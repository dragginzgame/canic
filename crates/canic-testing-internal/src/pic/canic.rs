use std::path::{Path, PathBuf};

use candid::{Principal, encode_one};
use canic::{
    Error,
    dto::{
        fleet_activation::{
            FleetActivationPhase, FleetActivationResumeRequest, FleetActivationStatusResponse,
        },
        fleet_subnet_root::{
            FleetSubnetRootAuthority, FleetSubnetRootInitArgs, FleetSubnetWasmStoreAdoptionRequest,
            FleetSubnetWasmStoreAdoptionResponse, FleetSubnetWasmStoreInitArgs,
        },
    },
    ids::{
        CanisterRole, ComponentSpecAdmission, CyclesFundingBudget, FleetCoordinatorBinding,
        FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding,
        FleetSubnetRootLimits, FleetSubnetRootReleaseSet, FleetSubnetWasmStoreAuthority,
        ReleaseSetDigest, SubnetId,
    },
    protocol,
};
use canic_core::{
    cdk::{types::Cycles, utils::hash::wasm_hash},
    ids::{AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, ReleaseBuildId},
};
use canic_host::release_set::AppConfigSnapshot;
use ic_testkit::{
    artifacts::{read_wasm, test_target_dir, workspace_root_for},
    pic::{
        CandidCallError, CandidCallExt, CanisterInstallExt, InstallSpec, PocketIc, PocketIcBuilder,
        PocketIcDiagnosticsExt, StandaloneCanisterFixture,
    },
};

use super::artifacts::{
    CanicWasmBuildProfile, INTERNAL_TEST_RELEASE_BUILD_ID, build_internal_test_wasm_canisters,
};

const INSTALL_CYCLES: u128 = 500_000_000_000_000;
const STANDALONE_READY_TICK_LIMIT: usize = 60;

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

    /// Wait until one Canic canister reports `canic_ready`.
    fn wait_for_ready(&self, canister_id: Principal, tick_limit: usize, context: &str);

    /// Wait until all provided Canic canisters report `canic_ready`.
    fn wait_for_all_ready<I>(&self, canister_ids: I, tick_limit: usize, context: &str)
    where
        I: IntoIterator<Item = Principal>;
}

impl CanicPicExt for PocketIc {
    fn create_and_install_root_canister(
        &self,
        root_wasm: Vec<u8>,
        wasm_store_wasm: Vec<u8>,
        config_path: &Path,
    ) -> Result<Principal, Error> {
        let root_id = self.create_canister();
        self.add_cycles(root_id, INSTALL_CYCLES);
        let wasm_store = self.create_canister();
        self.add_cycles(wasm_store, INSTALL_CYCLES);
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
            install_id: root_args.install_id,
        };
        prepare_sibling_wasm_store_controllers(self, wasm_store, installation_controller, root_id);
        self.install_canister(
            wasm_store,
            wasm_store_wasm,
            encode_one(store_args)
                .map_err(|error| Error::internal(format!("encode Store init failed: {error}")))?,
            Some(installation_controller),
        );
        self.install_canister(
            root_id,
            root_wasm,
            encode_one(&root_args)
                .map_err(|error| Error::internal(format!("encode root init failed: {error}")))?,
            None,
        );
        adopt_sibling_wasm_store(self, root_id, &root_args);
        Ok(root_id)
    }

    fn wait_for_ready(&self, canister_id: Principal, tick_limit: usize, context: &str) {
        for _ in 0..tick_limit {
            self.tick();
            if fetch_ready(self, canister_id) {
                return;
            }
        }

        self.dump_canister_debug(canister_id, context);
        panic!("{context}: canister {canister_id} did not become ready after {tick_limit} ticks");
    }

    fn wait_for_all_ready<I>(&self, canister_ids: I, tick_limit: usize, context: &str)
    where
        I: IntoIterator<Item = Principal>,
    {
        let canister_ids = canister_ids.into_iter().collect::<Vec<_>>();

        for _ in 0..tick_limit {
            self.tick();
            if canister_ids
                .iter()
                .copied()
                .all(|canister_id| fetch_ready(self, canister_id))
            {
                return;
            }
        }

        for canister_id in &canister_ids {
            self.dump_canister_debug(*canister_id, context);
        }
        panic!("{context}: canisters did not become ready after {tick_limit} ticks");
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
        operation_id: root_args.install_id,
        authority: root_args.authority.wasm_store_authority.clone(),
    };
    let adopted: Result<FleetSubnetWasmStoreAdoptionResponse, Error> = pic
        .update_candid(
            root,
            protocol::CANIC_FLEET_SUBNET_WASM_STORE_ADOPT,
            (request.clone(),),
        )
        .expect("adopt sibling Wasm Store transport");
    let adopted = adopted.expect("adopt sibling Wasm Store application");
    let status: Result<Option<FleetSubnetWasmStoreAdoptionResponse>, Error> = pic
        .query_candid(
            root,
            protocol::CANIC_FLEET_SUBNET_WASM_STORE_ADOPTION_STATUS,
            (request,),
        )
        .expect("query sibling Wasm Store adoption transport");
    assert_eq!(
        status.expect("query sibling Wasm Store adoption application"),
        Some(adopted)
    );
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
    let prepared: Result<FleetActivationStatusResponse, Error> = pic
        .update_candid(root_id, protocol::CANIC_PREPARE_FLEET_ACTIVATION, ())
        .expect("Fleet activation preparation transport");
    let prepared = prepared.unwrap_or_else(|error| {
        pic.dump_canister_debug(root_id, "Fleet activation preparation application");
        panic!("Fleet activation preparation application: {error:?}");
    });
    assert_eq!(prepared.phase, FleetActivationPhase::Prepared);
    let credential = prepared
        .credential
        .expect("prepared root must expose its credential generation");

    let activated: Result<FleetActivationStatusResponse, Error> = pic
        .update_candid(
            root_id,
            protocol::CANIC_RESUME_FLEET_ACTIVATION,
            (FleetActivationResumeRequest {
                operation_id: prepared.identity.operation_id,
                credential,
            },),
        )
        .expect("Fleet activation resume transport");
    let activated = activated.expect("Fleet activation resume application");
    assert_eq!(activated.phase, FleetActivationPhase::Active);
    activated
}

/// Wait until one Canic canister reports `canic_ready`.
///
/// # Panics
///
/// Panics if the canister does not report ready within `tick_limit` ticks, or
/// if querying readiness traps.
pub fn wait_until_ready(pic: &PocketIc, canister_id: Principal, tick_limit: usize) {
    for _ in 0..tick_limit {
        if let Ok(ready) = pic.query_candid_as::<bool, _>(
            canister_id,
            Principal::anonymous(),
            protocol::CANIC_READY,
            (),
        ) && ready
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

    let workspace_root = workspace_root();
    let target_name = format!("standalone-{crate_name}");
    let target_dir = test_target_dir(&workspace_root, &target_name);
    ensure_canister_wasm_ready(&workspace_root, &target_dir, crate_name, profile);

    let label = format!("standalone:{crate_name}:{role}");
    let wasm = read_wasm(&target_dir, crate_name, profile.target_dir_name());
    let pocket_ic = PocketIcBuilder::new().with_application_subnet().build();
    let fixture = StandaloneCanisterFixture::install(
        pocket_ic,
        InstallSpec::new(wasm, local_init_args(), 0).label(label),
    );
    let canister_id = fixture.canister_id();
    let pic = fixture.pocket_ic();
    pic.wait_for_ready(
        canister_id,
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

    let workspace_root = workspace_root();
    let target_name = format!("standalone-{crate_name}");
    let target_dir = test_target_dir(&workspace_root, &target_name);
    ensure_canister_wasm_ready(&workspace_root, &target_dir, crate_name, profile);

    let wasm = read_wasm(&target_dir, crate_name, profile.target_dir_name());
    let canister_id = pic
        .create_and_install(InstallSpec::new(wasm, local_init_args(), 0).label(label.to_string()));
    pic.wait_for_ready(
        canister_id,
        STANDALONE_READY_TICK_LIMIT,
        "standalone canister bootstrap",
    );

    canister_id
}

fn fetch_ready(pic: &PocketIc, canister_id: Principal) -> bool {
    match pic.query_candid(canister_id, protocol::CANIC_READY, ()) {
        Ok(ready) => ready,
        Err(err) => {
            let activation: Result<Result<FleetActivationStatusResponse, Error>, CandidCallError> =
                pic.query_candid(canister_id, protocol::CANIC_FLEET_ACTIVATION_STATUS, ());
            if matches!(
                activation,
                Ok(Ok(FleetActivationStatusResponse {
                    phase: FleetActivationPhase::Prepared,
                    ..
                }))
            ) {
                return false;
            }
            pic.dump_canister_debug(canister_id, "query canic_ready failed");
            panic!("query canic_ready failed: {err:?}");
        }
    }
}

pub(super) fn install_root_args_with_release_set_digest_and_coordinator(
    input: ManagedRootInstallInput<'_>,
) -> Result<Vec<u8>, Error> {
    encode_one(managed_test_root_init_args(input)?)
        .map_err(|err| Error::internal(format!("encode_one failed: {err}")))
}

fn ensure_canister_wasm_ready(
    workspace_root: &Path,
    target_dir: &Path,
    crate_name: &str,
    profile: CanicWasmBuildProfile,
) {
    build_internal_test_wasm_canisters(workspace_root, target_dir, &[crate_name], profile);
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
        .map_err(|error| Error::internal(format!("load root test config failed: {error}")))?;
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
        .map_err(|error| Error::internal(format!("compile root test authority failed: {error}")))?;
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

    Ok(FleetSubnetRootInitArgs {
        authority: FleetSubnetRootAuthority {
            binding: FleetSubnetRootBinding {
                authority: authority.clone(),
                placement_subnet: test_subnet(0x45),
                fleet_subnet_root: root_id,
                component_admissions,
                component_topology_digest,
                limits: FleetSubnetRootLimits {
                    maximum_component_instances: 4_096,
                    maximum_registry_bytes: 16_777_216,
                    maximum_wasm_store_bytes: 536_870_912,
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
                authority,
                placement_subnet: test_subnet(0x45),
                fleet_subnet_root: root_id,
                wasm_store,
                installation_controller,
                release_build_id: identity.release_build_id,
                wasm_module_hash: expected_wasm_store_module_hash,
            },
        },
        install_id: identity.install_id,
        canister_pool_imports: Vec::new(),
    })
}

const fn test_subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(Principal::from_slice(&[byte; 29]))
}

fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}
