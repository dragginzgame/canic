use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use candid::{Principal, encode_args, encode_one};
use canic::{
    Error,
    dto::{
        abi::v1::CanisterInitPayload,
        env::EnvBootstrapArgs,
        subnet::SubnetIdentity,
        topology::{AppIndexArgs, SubnetIndexArgs, SubnetRegistryResponse},
    },
    ids::{CanisterRole, SubnetRole},
    protocol,
};
use ic_testkit::{
    Fake,
    artifacts::{read_wasm, test_target_dir, workspace_root_for},
    pic::{InstallSpec, Pic, StandaloneCanisterFixture, install_prebuilt_canister_from_spec},
};

use super::artifacts::{CanicWasmBuildProfile, build_internal_test_wasm_canisters};

const INSTALL_CYCLES: u128 = 500_000_000_000_000;
const STANDALONE_READY_TICK_LIMIT: usize = 60;
static STANDALONE_BUILD_SERIAL: Mutex<()> = Mutex::new(());

///
/// CanicPicExt
///

pub trait CanicPicExt {
    /// Install a root Canic canister with the default root init arguments.
    fn create_and_install_root_canister(&self, wasm: Vec<u8>) -> Result<Principal, Error>;

    /// Install a non-root Canic canister with default standalone init arguments.
    fn create_and_install_canister(
        &self,
        role: CanisterRole,
        wasm: Vec<u8>,
    ) -> Result<Principal, Error>;

    /// Wait until one Canic canister reports `canic_ready`.
    fn wait_for_ready(&self, canister_id: Principal, tick_limit: usize, context: &str);

    /// Wait until all provided Canic canisters report `canic_ready`.
    fn wait_for_all_ready<I>(&self, canister_ids: I, tick_limit: usize, context: &str)
    where
        I: IntoIterator<Item = Principal>;
}

impl CanicPicExt for Pic {
    fn create_and_install_root_canister(&self, wasm: Vec<u8>) -> Result<Principal, Error> {
        let init_bytes = install_root_args()?;

        Ok(self
            .create_and_install(InstallSpec::new(wasm, init_bytes, INSTALL_CYCLES).label("root")))
    }

    fn create_and_install_canister(
        &self,
        role: CanisterRole,
        wasm: Vec<u8>,
    ) -> Result<Principal, Error> {
        let label = role.to_string();
        let init_bytes = install_args(role)?;

        Ok(
            self.create_and_install(
                InstallSpec::new(wasm, init_bytes, INSTALL_CYCLES).label(label),
            ),
        )
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
/// The installed canister receives explicit local env bootstrap fields, empty
/// topology indexes, and the internal test endpoint surface for that test
/// build.
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
        InstallSpec::new(wasm, standalone_init_args(role), 0).label(label),
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
    let canister_id = pic.create_and_install(
        InstallSpec::new(wasm, standalone_init_args(role), 0).label(label.to_string()),
    );
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
            pic.dump_canister_debug(canister_id, "query canic_ready failed");
            panic!("query canic_ready failed: {err:?}");
        }
    }
}

fn install_args(role: CanisterRole) -> Result<Vec<u8>, Error> {
    if role.is_root() {
        install_root_args()
    } else {
        let env = EnvBootstrapArgs {
            prime_root_pid: None,
            subnet_role: None,
            subnet_pid: None,
            root_pid: None,
            canister_role: Some(role),
            parent_pid: None,
        };

        let payload = CanisterInitPayload {
            env,
            app_index: AppIndexArgs(Vec::new()),
            subnet_index: SubnetIndexArgs(Vec::new()),
        };

        encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
            .map_err(|err| Error::internal(format!("encode_args failed: {err}")))
    }
}

fn install_root_args() -> Result<Vec<u8>, Error> {
    encode_one(SubnetIdentity::Manual)
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

fn standalone_init_args(role: CanisterRole) -> Vec<u8> {
    let root_pid = Fake::principal(1);
    let payload = CanisterInitPayload {
        env: EnvBootstrapArgs {
            prime_root_pid: Some(root_pid),
            subnet_role: Some(SubnetRole::PRIME),
            subnet_pid: Some(Fake::principal(2)),
            root_pid: Some(root_pid),
            canister_role: Some(role),
            parent_pid: Some(root_pid),
        },
        app_index: AppIndexArgs(Vec::new()),
        subnet_index: SubnetIndexArgs(Vec::new()),
    };

    encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
        .expect("encode standalone init args")
}

fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}
