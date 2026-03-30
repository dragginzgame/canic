use candid::{CandidType, Principal, decode_one, encode_args, encode_one, utils::ArgumentEncoder};
use canic::{
    Error,
    cdk::types::TC,
    dto::{
        abi::v1::CanisterInitPayload,
        env::EnvBootstrapArgs,
        subnet::SubnetIdentity,
        topology::{AppDirectoryArgs, SubnetDirectoryArgs},
    },
    ids::CanisterRole,
    protocol,
};
use pocket_ic::{PocketIc, PocketIcBuilder};
use serde::de::DeserializeOwned;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Mutex, MutexGuard},
};

const INSTALL_CYCLES: u128 = 500 * TC;
static PIC_BUILD_SERIAL: Mutex<()> = Mutex::new(());

struct ControllerSnapshot {
    snapshot_id: Vec<u8>,
    sender: Option<Principal>,
}

///
/// ControllerSnapshots
///

pub struct ControllerSnapshots(HashMap<Principal, ControllerSnapshot>);

///
/// Create a fresh PocketIC universe.
///
/// IMPORTANT:
/// - Each call creates a new IC instance
/// - WARNING: DO NOT CACHE OR SHARE `Pic` ACROSS TESTS
/// - Reusing `Pic` can retain global locks and background runtime state
///   and can make later tests hang or fail nondeterministically
/// - Required to avoid PocketIC wasm chunk store exhaustion
///
#[must_use]
pub fn pic() -> Pic {
    PicBuilder::new().with_application_subnet().build()
}

///
/// PicBuilder
/// Thin wrapper around the PocketIC builder.
///
/// This builder is only used to configure the singleton. It does not create
/// additional IC instances beyond the global `Pic`.
///
/// Note: this file is test-only infrastructure; simplicity wins over abstraction.
///

pub struct PicBuilder(PocketIcBuilder);

#[expect(clippy::new_without_default)]
impl PicBuilder {
    /// Start a new PicBuilder with sensible defaults.
    #[must_use]
    pub fn new() -> Self {
        Self(PocketIcBuilder::new())
    }

    /// Include an application subnet in the singleton universe.
    #[must_use]
    pub fn with_application_subnet(mut self) -> Self {
        self.0 = self.0.with_application_subnet();
        self
    }

    /// Include an NNS subnet in the singleton universe.
    #[must_use]
    pub fn with_nns_subnet(mut self) -> Self {
        self.0 = self.0.with_nns_subnet();
        self
    }

    /// Finish building the singleton PocketIC instance and wrap it.
    #[must_use]
    pub fn build(self) -> Pic {
        // Hold the guard for the full PocketIC lifetime to avoid concurrent
        // server interactions that can crash the local pocket-ic process
        // (for example `KeyAlreadyExists { key: "nns_subnet_id", ... }`).
        let serial_guard = PIC_BUILD_SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        Pic {
            inner: self.0.build(),
            _serial_guard: serial_guard,
        }
    }
}

///
/// Pic
/// Thin wrapper around the global PocketIC instance.
///
/// This type intentionally exposes only a minimal API surface; callers should
/// use `pic()` to obtain the singleton and then perform installs/calls.
///
/// WARNING: DO NOT CACHE OR SHARE `Pic` ACROSS TESTS.
/// Keep `Pic` lifetime scoped to a single test setup and drop it promptly.
///

pub struct Pic {
    inner: PocketIc,
    _serial_guard: MutexGuard<'static, ()>,
}

impl Pic {
    /// Install a root canister with the default root init arguments.
    pub fn create_and_install_root_canister(&self, wasm: Vec<u8>) -> Result<Principal, Error> {
        let init_bytes = install_root_args()?;

        Ok(self.create_funded_and_install(wasm, init_bytes))
    }

    /// Install a canister with the given type and wasm bytes.
    ///
    /// Install failures are treated as fatal in tests.
    pub fn create_and_install_canister(
        &self,
        role: CanisterRole,
        wasm: Vec<u8>,
    ) -> Result<Principal, Error> {
        let init_bytes = install_args(role)?;

        Ok(self.create_funded_and_install(wasm, init_bytes))
    }

    /// Wait until one canister reports `canic_ready`.
    pub fn wait_for_ready(&self, canister_id: Principal, tick_limit: usize, context: &str) {
        for _ in 0..tick_limit {
            self.tick();
            if self.fetch_ready(canister_id) {
                return;
            }
        }

        self.dump_canister_debug(canister_id, context);
        panic!("{context}: canister {canister_id} did not become ready after {tick_limit} ticks");
    }

    /// Wait until all provided canisters report `canic_ready`.
    pub fn wait_for_all_ready<I>(&self, canister_ids: I, tick_limit: usize, context: &str)
    where
        I: IntoIterator<Item = Principal>,
    {
        let canister_ids = canister_ids.into_iter().collect::<Vec<_>>();

        for _ in 0..tick_limit {
            self.tick();
            if canister_ids
                .iter()
                .copied()
                .all(|canister_id| self.fetch_ready(canister_id))
            {
                return;
            }
        }

        for canister_id in &canister_ids {
            self.dump_canister_debug(*canister_id, context);
        }
        panic!("{context}: canisters did not become ready after {tick_limit} ticks");
    }

    /// Dump basic PocketIC status and log context for one canister.
    pub fn dump_canister_debug(&self, canister_id: Principal, context: &str) {
        eprintln!("{context}: debug for canister {canister_id}");

        match self.canister_status(canister_id, None) {
            Ok(status) => eprintln!("canister_status: {status:?}"),
            Err(err) => eprintln!("canister_status failed: {err:?}"),
        }

        match self.fetch_canister_logs(canister_id, Principal::anonymous()) {
            Ok(records) => {
                if records.is_empty() {
                    eprintln!("canister logs: <empty>");
                } else {
                    for record in records {
                        eprintln!("canister log: {record:?}");
                    }
                }
            }
            Err(err) => eprintln!("fetch_canister_logs failed: {err:?}"),
        }
    }

    /// Capture one restorable snapshot per canister using a shared controller.
    pub fn capture_controller_snapshots<I>(
        &self,
        controller_id: Principal,
        canister_ids: I,
    ) -> Option<ControllerSnapshots>
    where
        I: IntoIterator<Item = Principal>,
    {
        let mut snapshots = HashMap::new();

        for canister_id in canister_ids {
            let Some(snapshot) = self.try_take_controller_snapshot(controller_id, canister_id)
            else {
                eprintln!(
                    "capture_controller_snapshots: snapshot capture unavailable for {canister_id}"
                );
                return None;
            };
            snapshots.insert(canister_id, snapshot);
        }

        Some(ControllerSnapshots(snapshots))
    }

    /// Restore a previously captured snapshot set using the same controller.
    pub fn restore_controller_snapshots(
        &self,
        controller_id: Principal,
        snapshots: &ControllerSnapshots,
    ) {
        for (canister_id, snapshot) in &snapshots.0 {
            self.restore_controller_snapshot(controller_id, *canister_id, snapshot);
        }
    }

    /// Generic update call helper (serializes args + decodes result).
    pub fn update_call<T, A>(
        &self,
        canister_id: Principal,
        method: &str,
        args: A,
    ) -> Result<T, Error>
    where
        T: CandidType + DeserializeOwned,
        A: ArgumentEncoder,
    {
        let bytes: Vec<u8> = encode_args(args)
            .map_err(|err| Error::internal(format!("encode_args failed: {err}")))?;
        let result = self
            .inner
            .update_call(canister_id, Principal::anonymous(), method, bytes)
            .map_err(|err| {
                Error::internal(format!(
                    "pocket_ic update_call failed (canister={canister_id}, method={method}): {err}"
                ))
            })?;

        decode_one(&result).map_err(|err| Error::internal(format!("decode_one failed: {err}")))
    }

    /// Generic update call helper with an explicit caller principal.
    pub fn update_call_as<T, A>(
        &self,
        canister_id: Principal,
        caller: Principal,
        method: &str,
        args: A,
    ) -> Result<T, Error>
    where
        T: CandidType + DeserializeOwned,
        A: ArgumentEncoder,
    {
        let bytes: Vec<u8> = encode_args(args)
            .map_err(|err| Error::internal(format!("encode_args failed: {err}")))?;
        let result = self
            .inner
            .update_call(canister_id, caller, method, bytes)
            .map_err(|err| {
                Error::internal(format!(
                    "pocket_ic update_call failed (canister={canister_id}, method={method}): {err}"
                ))
            })?;

        decode_one(&result).map_err(|err| Error::internal(format!("decode_one failed: {err}")))
    }

    /// Generic query call helper.
    pub fn query_call<T, A>(
        &self,
        canister_id: Principal,
        method: &str,
        args: A,
    ) -> Result<T, Error>
    where
        T: CandidType + DeserializeOwned,
        A: ArgumentEncoder,
    {
        let bytes: Vec<u8> = encode_args(args)
            .map_err(|err| Error::internal(format!("encode_args failed: {err}")))?;
        let result = self
            .inner
            .query_call(canister_id, Principal::anonymous(), method, bytes)
            .map_err(|err| {
                Error::internal(format!(
                    "pocket_ic query_call failed (canister={canister_id}, method={method}): {err}"
                ))
            })?;

        decode_one(&result).map_err(|err| Error::internal(format!("decode_one failed: {err}")))
    }

    /// Generic query call helper with an explicit caller principal.
    pub fn query_call_as<T, A>(
        &self,
        canister_id: Principal,
        caller: Principal,
        method: &str,
        args: A,
    ) -> Result<T, Error>
    where
        T: CandidType + DeserializeOwned,
        A: ArgumentEncoder,
    {
        let bytes: Vec<u8> = encode_args(args)
            .map_err(|err| Error::internal(format!("encode_args failed: {err}")))?;
        let result = self
            .inner
            .query_call(canister_id, caller, method, bytes)
            .map_err(|err| {
                Error::internal(format!(
                    "pocket_ic query_call failed (canister={canister_id}, method={method}): {err}"
                ))
            })?;

        decode_one(&result).map_err(|err| Error::internal(format!("decode_one failed: {err}")))
    }

    /// Advance PocketIC by a fixed number of ticks.
    pub fn tick_n(&self, times: usize) {
        for _ in 0..times {
            self.tick();
        }
    }

    // Install a canister after creating it and funding it with cycles.
    fn create_funded_and_install(&self, wasm: Vec<u8>, init_bytes: Vec<u8>) -> Principal {
        let canister_id = self.create_canister();
        self.add_cycles(canister_id, INSTALL_CYCLES);

        let install = catch_unwind(AssertUnwindSafe(|| {
            self.inner
                .install_canister(canister_id, wasm, init_bytes, None);
        }));
        if let Err(err) = install {
            eprintln!("install_canister trapped for {canister_id}");
            if let Ok(status) = self.inner.canister_status(canister_id, None) {
                eprintln!("canister_status for {canister_id}: {status:?}");
            }
            if let Ok(logs) = self
                .inner
                .fetch_canister_logs(canister_id, Principal::anonymous())
            {
                for record in logs {
                    eprintln!("canister_log {canister_id}: {record:?}");
                }
            }
            std::panic::resume_unwind(err);
        }

        canister_id
    }

    // Query `canic_ready` and panic with debug context on transport failures.
    fn fetch_ready(&self, canister_id: Principal) -> bool {
        match self.query_call(canister_id, protocol::CANIC_READY, ()) {
            Ok(ready) => ready,
            Err(err) => {
                self.dump_canister_debug(canister_id, "query canic_ready failed");
                panic!("query canic_ready failed: {err:?}");
            }
        }
    }

    // Capture one snapshot with sender fallbacks that match controller ownership.
    fn try_take_controller_snapshot(
        &self,
        controller_id: Principal,
        canister_id: Principal,
    ) -> Option<ControllerSnapshot> {
        let candidates = controller_sender_candidates(controller_id, canister_id);
        let mut last_err = None;

        for sender in candidates {
            match self.take_canister_snapshot(canister_id, sender, None) {
                Ok(snapshot) => {
                    return Some(ControllerSnapshot {
                        snapshot_id: snapshot.id,
                        sender,
                    });
                }
                Err(err) => last_err = Some((sender, err)),
            }
        }

        if let Some((sender, err)) = last_err {
            eprintln!(
                "failed to capture canister snapshot for {canister_id} using sender {sender:?}: {err}"
            );
        }
        None
    }

    // Restore one snapshot with sender fallbacks that match controller ownership.
    fn restore_controller_snapshot(
        &self,
        controller_id: Principal,
        canister_id: Principal,
        snapshot: &ControllerSnapshot,
    ) {
        let fallback_sender = if snapshot.sender.is_some() {
            None
        } else {
            Some(controller_id)
        };
        let candidates = [snapshot.sender, fallback_sender];
        let mut last_err = None;

        for sender in candidates {
            match self.load_canister_snapshot(canister_id, sender, snapshot.snapshot_id.clone()) {
                Ok(()) => return,
                Err(err) => last_err = Some((sender, err)),
            }
        }

        let (sender, err) =
            last_err.expect("snapshot restore must have at least one sender attempt");
        panic!(
            "failed to restore canister snapshot for {canister_id} using sender {sender:?}: {err}"
        );
    }
}

impl Deref for Pic {
    type Target = PocketIc;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Pic {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// --------------------------------------
/// install_args helper
/// --------------------------------------
///
/// Init semantics:
/// - Root canisters receive a `SubnetIdentity` (direct root bootstrap).
/// - Non-root canisters receive `EnvBootstrapArgs` + optional directory snapshots.
///
/// Directory handling:
/// - By default, directory views are empty for standalone installs.
/// - Directory-dependent logic is opt-in via `install_args_with_directories`.
/// - Root-provisioned installs will populate directories via cascade.
///

fn install_args(role: CanisterRole) -> Result<Vec<u8>, Error> {
    if role.is_root() {
        install_root_args()
    } else {
        // Non-root standalone install.
        // Provide only what is structurally known at install time.
        let env = EnvBootstrapArgs {
            prime_root_pid: None,
            subnet_role: None,
            subnet_pid: None,
            root_pid: None,
            canister_role: Some(role),
            parent_pid: None,
        };

        // Intentional: standalone installs do not require directories unless
        // a test explicitly exercises directory-dependent behavior.
        let payload = CanisterInitPayload {
            env,
            app_directory: AppDirectoryArgs(Vec::new()),
            subnet_directory: SubnetDirectoryArgs(Vec::new()),
        };

        encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
            .map_err(|err| Error::internal(format!("encode_args failed: {err}")))
    }
}

fn install_root_args() -> Result<Vec<u8>, Error> {
    encode_one(SubnetIdentity::Manual)
        .map_err(|err| Error::internal(format!("encode_one failed: {err}")))
}

// Prefer the likely controller sender first to reduce noisy management-call failures.
fn controller_sender_candidates(
    controller_id: Principal,
    canister_id: Principal,
) -> [Option<Principal>; 2] {
    if canister_id == controller_id {
        [None, Some(controller_id)]
    } else {
        [Some(controller_id), None]
    }
}
