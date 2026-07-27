use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use candid::{Principal, encode_one};
use canic::{
    Error,
    dto::{
        fleet_activation::{
            FleetActivationPhase, FleetActivationResumeRequest, FleetActivationStatusResponse,
        },
        fleet_subnet_root::{FleetSubnetRootAuthority, FleetSubnetRootInitArgs},
        topology::SubnetRegistryResponse,
    },
    ids::{
        CanisterRole, ComponentSpecAdmission, CyclesFundingBudget, FleetCoordinatorBinding,
        FleetRegistryAuthority, FleetSubnetRootBinding, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, ReleaseSetDigest, SubnetId,
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
    pic::{InstallSpec, Pic, StandaloneCanisterFixture, install_prebuilt_canister_from_spec},
};

use super::artifacts::{
    CanicWasmBuildProfile, INTERNAL_TEST_RELEASE_BUILD_ID, build_internal_test_wasm_canisters,
};

const INSTALL_CYCLES: u128 = 500_000_000_000_000;
const STANDALONE_READY_TICK_LIMIT: usize = 60;
static STANDALONE_BUILD_SERIAL: Mutex<()> = Mutex::new(());

///
/// CanicPicExt
///

pub trait CanicPicExt {
    /// Install a root Canic canister with authority compiled from its exact build config.
    fn create_and_install_root_canister(
        &self,
        wasm: Vec<u8>,
        config_path: &Path,
    ) -> Result<Principal, Error>;

    /// Wait until one Canic canister reports `canic_ready`.
    fn wait_for_ready(&self, canister_id: Principal, tick_limit: usize, context: &str);

    /// Wait until all provided Canic canisters report `canic_ready`.
    fn wait_for_all_ready<I>(&self, canister_ids: I, tick_limit: usize, context: &str)
    where
        I: IntoIterator<Item = Principal>;
}

impl CanicPicExt for Pic {
    fn create_and_install_root_canister(
        &self,
        wasm: Vec<u8>,
        config_path: &Path,
    ) -> Result<Principal, Error> {
        let root_id = self.create_canister();
        self.add_cycles(root_id, INSTALL_CYCLES);
        let init_bytes = install_root_args(root_id, &wasm, config_path)?;
        self.install_canister(root_id, wasm, init_bytes, None);
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

/// Drive one prepared managed Fleet through the exact controller activation protocol.
///
/// # Panics
///
/// Panics when preparation or activation transport fails, the root rejects
/// either operation, or the resulting phase is not the exact expected phase.
pub(super) fn activate_managed_fleet(
    pic: &Pic,
    root_id: Principal,
) -> FleetActivationStatusResponse {
    let prepared: Result<FleetActivationStatusResponse, Error> = pic
        .update_call(root_id, protocol::CANIC_PREPARE_FLEET_ACTIVATION, ())
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
        .update_call(
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
pub fn wait_until_ready(pic: &Pic, canister_id: Principal, tick_limit: usize) {
    for _ in 0..tick_limit {
        if let Ok(ready) = pic.query_call_as::<bool, _>(
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

/// Resolve one role principal from root's subnet registry, polling until present.
///
/// # Panics
///
/// Panics if the requested role is not present in root's subnet registry within
/// `tick_limit` ticks.
#[must_use]
pub fn role_pid(pic: &Pic, root_id: Principal, role: &'static str, tick_limit: usize) -> Principal {
    for _ in 0..tick_limit {
        let registry: Result<Result<SubnetRegistryResponse, Error>, _> = pic.query_call_as(
            root_id,
            Principal::anonymous(),
            protocol::CANIC_SUBNET_REGISTRY,
            (),
        );

        if let Ok(Ok(registry)) = registry
            && let Some(pid) = registry
                .0
                .into_iter()
                .find(|entry| entry.role == CanisterRole::new(role))
                .map(|entry| entry.pid)
        {
            return pid;
        }

        pic.tick();
    }

    panic!("{role} canister must be registered");
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
    let fixture = install_prebuilt_canister_from_spec(
        InstallSpec::new(wasm, local_init_args(), 0).label(label),
    );
    let canister_id = fixture.canister_id();
    let pic = fixture.pic();
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
    pic: &Pic,
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

fn fetch_ready(pic: &Pic, canister_id: Principal) -> bool {
    match pic.query_call(canister_id, protocol::CANIC_READY, ()) {
        Ok(ready) => ready,
        Err(err) => {
            let activation: Result<
                Result<FleetActivationStatusResponse, Error>,
                ic_testkit::pic::PicCallError,
            > = pic.query_call(canister_id, protocol::CANIC_FLEET_ACTIVATION_STATUS, ());
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

pub fn install_root_args(
    root_id: Principal,
    wasm: &[u8],
    config_path: &Path,
) -> Result<Vec<u8>, Error> {
    install_root_args_with_release_set_digest(
        root_id,
        wasm,
        config_path,
        ReleaseSetDigest::from_bytes([0x44; 32]),
    )
}

pub(super) fn install_root_args_with_release_set_digest(
    root_id: Principal,
    wasm: &[u8],
    config_path: &Path,
    release_set_digest: ReleaseSetDigest,
) -> Result<Vec<u8>, Error> {
    encode_one(managed_test_root_init_args(
        root_id,
        wasm,
        config_path,
        release_set_digest,
    )?)
    .map_err(|err| Error::internal(format!("encode_one failed: {err}")))
}

fn ensure_canister_wasm_ready(
    workspace_root: &Path,
    target_dir: &Path,
    crate_name: &str,
    profile: CanicWasmBuildProfile,
) {
    let _build_guard = STANDALONE_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

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
                canonical_network_id: CanonicalNetworkId::public_ic(),
                fleet_id: FleetId::from_generated_bytes([0x42; 32]),
            },
            app: AppId::from("canic-internal-test"),
        },
        install_id: [0x43; 32],
        release_build_id,
    }
}

fn managed_test_root_init_args(
    root_id: Principal,
    wasm: &[u8],
    config_path: &Path,
    release_set_digest: ReleaseSetDigest,
) -> Result<FleetSubnetRootInitArgs, Error> {
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
    let expected_module_hash =
        <[u8; 32]>::try_from(wasm_hash(wasm)).expect("SHA-256 helper must return exactly 32 bytes");
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: identity.fleet.fleet,
                app: AppId::from(config.app_id()),
            },
            coordinator_subnet: test_subnet(0x40),
            coordinator: Principal::from_slice(&[0x41; 29]),
        },
        epoch: 1,
    };

    Ok(FleetSubnetRootInitArgs {
        authority: FleetSubnetRootAuthority {
            binding: FleetSubnetRootBinding {
                authority,
                placement_subnet: test_subnet(0x45),
                fleet_subnet_root: root_id,
                component_admissions,
                component_topology_digest,
                limits: FleetSubnetRootLimits {
                    maximum_component_instances: 4_096,
                    maximum_managed_canisters: 25_000,
                    maximum_registry_bytes: 16_777_216,
                    maximum_wasm_store_bytes: 536_870_912,
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
        },
        install_id: identity.install_id,
    })
}

const fn test_subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(Principal::from_slice(&[byte; 29]))
}

fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}
