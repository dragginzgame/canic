use candid::{Principal, decode_one, encode_args, encode_one};
use canic::{
    Error,
    cdk::types::TC,
    dto::{
        abi::v1::CanisterInitPayload,
        env::EnvBootstrapArgs,
        subnet::SubnetIdentity,
        topology::{AppDirectoryArgs, SubnetDirectoryArgs, SubnetRegistryResponse},
    },
    ids::CanisterRole,
    protocol,
};
use pocket_ic::{PocketIc, PocketIcBuilder};
use std::{
    ops::{Deref, DerefMut},
    panic::AssertUnwindSafe,
};

mod baseline;
mod calls;
mod diagnostics;
mod lifecycle;
mod process_lock;
mod snapshot;
mod standalone;
mod startup;

pub use baseline::{
    CachedPicBaseline, CachedPicBaselineGuard, ControllerSnapshots, acquire_cached_pic_baseline,
    drop_stale_cached_pic_baseline, restore_or_rebuild_cached_pic_baseline,
};
pub use process_lock::{
    PicSerialGuard, PicSerialGuardError, acquire_pic_serial_guard, try_acquire_pic_serial_guard,
};
pub use startup::PicStartError;
const INSTALL_CYCLES: u128 = 500 * TC;

///
/// PicInstallError
///

#[derive(Debug, Eq, PartialEq)]
pub struct PicInstallError {
    canister_id: Principal,
    message: String,
}

///
/// StandaloneCanisterFixtureError
///

#[derive(Debug)]
pub enum StandaloneCanisterFixtureError {
    SerialGuard(PicSerialGuardError),
    Start(PicStartError),
    Install(PicInstallError),
}

pub use standalone::{
    StandaloneCanisterFixture, install_prebuilt_canister, install_prebuilt_canister_with_cycles,
    install_standalone_canister, try_install_prebuilt_canister,
    try_install_prebuilt_canister_with_cycles,
};

///
/// Create a fresh PocketIC universe.
///
/// IMPORTANT:
/// - Each call creates a new IC instance
/// - WARNING: callers must hold a `PicSerialGuard` for the full `Pic` lifetime
/// - Required to avoid PocketIC wasm chunk store exhaustion
///
#[must_use]
pub fn pic() -> Pic {
    try_pic().unwrap_or_else(|err| panic!("failed to start PocketIC: {err}"))
}

/// Create a fresh PocketIC universe without panicking on startup failures.
pub fn try_pic() -> Result<Pic, PicStartError> {
    PicBuilder::new().with_application_subnet().try_build()
}

/// Wait until a PocketIC canister reports `canic_ready`.
pub fn wait_until_ready(pic: &PocketIc, canister_id: Principal, tick_limit: usize) {
    let payload = encode_args(()).expect("encode empty args");

    for _ in 0..tick_limit {
        if let Ok(bytes) = pic.query_call(
            canister_id,
            Principal::anonymous(),
            protocol::CANIC_READY,
            payload.clone(),
        ) && let Ok(ready) = decode_one::<bool>(&bytes)
            && ready
        {
            return;
        }
        pic.tick();
    }

    panic!("canister did not report ready in time: {canister_id}");
}

/// Resolve one role principal from root's subnet registry, polling until present.
#[must_use]
pub fn role_pid(
    pic: &PocketIc,
    root_id: Principal,
    role: &'static str,
    tick_limit: usize,
) -> Principal {
    for _ in 0..tick_limit {
        let registry: Result<Result<SubnetRegistryResponse, Error>, Error> = {
            let payload = encode_args(()).expect("encode empty args");
            pic.query_call(
                root_id,
                Principal::anonymous(),
                protocol::CANIC_SUBNET_REGISTRY,
                payload,
            )
            .map_err(|err| {
                Error::internal(format!(
                    "pocket_ic query_call failed (canister={root_id}, method={}): {err}",
                    protocol::CANIC_SUBNET_REGISTRY
                ))
            })
            .and_then(|bytes| {
                decode_one(&bytes).map_err(|err| {
                    Error::internal(format!("decode_one failed for subnet registry: {err}"))
                })
            })
        };

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

    /// Include an application subnet in the PocketIC universe.
    #[must_use]
    pub fn with_application_subnet(mut self) -> Self {
        self.0 = self.0.with_application_subnet();
        self
    }

    /// Include an II subnet so threshold keys are available in the PocketIC universe.
    #[must_use]
    pub fn with_ii_subnet(mut self) -> Self {
        self.0 = self.0.with_ii_subnet();
        self
    }

    /// Include an NNS subnet in the PocketIC universe.
    #[must_use]
    pub fn with_nns_subnet(mut self) -> Self {
        self.0 = self.0.with_nns_subnet();
        self
    }

    /// Finish building the PocketIC instance and wrap it.
    #[must_use]
    pub fn build(self) -> Pic {
        self.try_build()
            .unwrap_or_else(|err| panic!("failed to start PocketIC: {err}"))
    }

    /// Finish building the PocketIC instance without panicking on startup failures.
    pub fn try_build(self) -> Result<Pic, PicStartError> {
        startup::try_build_pic(AssertUnwindSafe(self.0).0)
    }
}

impl PicInstallError {
    /// Capture one install failure for a specific canister id.
    #[must_use]
    pub const fn new(canister_id: Principal, message: String) -> Self {
        Self {
            canister_id,
            message,
        }
    }

    /// Read the canister id that failed to install.
    #[must_use]
    pub const fn canister_id(&self) -> Principal {
        self.canister_id
    }

    /// Read the captured panic message from the install attempt.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for PicInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to install canister {}: {}",
            self.canister_id, self.message
        )
    }
}

impl std::error::Error for PicInstallError {}

impl std::fmt::Display for StandaloneCanisterFixtureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerialGuard(err) => write!(f, "{err}"),
            Self::Start(err) => write!(f, "{err}"),
            Self::Install(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for StandaloneCanisterFixtureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SerialGuard(err) => Some(err),
            Self::Start(err) => Some(err),
            Self::Install(err) => Some(err),
        }
    }
}

///
/// Pic
/// Thin wrapper around a PocketIC instance.
///
/// This type intentionally exposes only a minimal API surface; callers should
/// use `pic()` to obtain an instance and then perform installs/calls.
/// Callers must hold a `PicSerialGuard` for the full `Pic` lifetime.
///

pub struct Pic {
    inner: PocketIc,
}

impl Pic {
    /// Capture the current PocketIC wall-clock time as nanoseconds since epoch.
    #[must_use]
    pub fn current_time_nanos(&self) -> u64 {
        self.inner.get_time().as_nanos_since_unix_epoch()
    }

    /// Restore PocketIC wall-clock and certified time from a captured nanosecond value.
    pub fn restore_time_nanos(&self, nanos_since_epoch: u64) {
        let restored = pocket_ic::Time::from_nanos_since_unix_epoch(nanos_since_epoch);
        self.inner.set_time(restored);
        self.inner.set_certified_time(restored);
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
