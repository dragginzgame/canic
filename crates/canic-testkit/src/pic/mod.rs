//! PocketIC wrapper and fixture helpers for host-side canister tests.

use candid::Principal;
use pocket_ic::{
    CanisterStatusResult, PocketIc, PocketIcBuilder, RejectResponse, common::rest::RawMessageId,
};
use std::time::Duration;

mod baseline;
mod calls;
mod diagnostics;
mod errors;
mod lifecycle;
mod process_lock;
mod snapshot;
mod standalone;
mod startup;

pub use baseline::{
    CachedPicBaseline, CachedPicBaselineGuard, ControllerSnapshots,
    restore_or_rebuild_cached_pic_baseline,
};
pub use errors::{PicCallError, PicInstallError, StandaloneCanisterFixtureError};
pub use process_lock::{
    PicSerialGuard, PicSerialGuardError, acquire_pic_serial_guard, try_acquire_pic_serial_guard,
};
pub use startup::PicStartError;

pub use standalone::{
    StandaloneCanisterFixture, install_prebuilt_canister, install_prebuilt_canister_with_cycles,
    try_install_prebuilt_canister, try_install_prebuilt_canister_with_cycles,
};

///
/// Create a fresh PocketIC instance with the default application subnet layout.
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

/// Create a fresh PocketIC instance without panicking on startup failures.
pub fn try_pic() -> Result<Pic, PicStartError> {
    PicBuilder::new().with_application_subnet().try_build()
}

///
/// PicBuilder
/// Thin wrapper around the PocketIC builder.
///
/// This builder configures one PocketIC instance before startup.
/// It does not share or reuse a global test runtime.
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

    /// Include an application subnet in the PocketIC instance.
    #[must_use]
    pub fn with_application_subnet(mut self) -> Self {
        self.0 = self.0.with_application_subnet();
        self
    }

    /// Include an II subnet so threshold keys are available in the PocketIC instance.
    #[must_use]
    pub fn with_ii_subnet(mut self) -> Self {
        self.0 = self.0.with_ii_subnet();
        self
    }

    /// Include an NNS subnet in the PocketIC instance.
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
        startup::try_build_pic(self.0)
    }
}
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
    /// Advance one execution round in the owned PocketIC instance.
    pub fn tick(&self) {
        self.inner.tick();
    }

    /// Advance PocketIC wall-clock time by one duration.
    pub fn advance_time(&self, duration: Duration) {
        self.inner.advance_time(duration);
    }

    /// Create one canister with PocketIC default settings.
    #[must_use]
    pub fn create_canister(&self) -> Principal {
        self.inner.create_canister()
    }

    /// Add cycles to one existing canister.
    pub fn add_cycles(&self, canister_id: Principal, amount: u128) {
        let _ = self.inner.add_cycles(canister_id, amount);
    }

    /// Install one wasm module on one existing canister.
    pub fn install_canister(
        &self,
        canister_id: Principal,
        wasm_module: Vec<u8>,
        arg: Vec<u8>,
        sender: Option<Principal>,
    ) {
        self.inner
            .install_canister(canister_id, wasm_module, arg, sender);
    }

    /// Upgrade one existing canister with a new wasm module.
    pub fn upgrade_canister(
        &self,
        canister_id: Principal,
        wasm_module: Vec<u8>,
        arg: Vec<u8>,
        sender: Option<Principal>,
    ) -> Result<(), RejectResponse> {
        self.inner
            .upgrade_canister(canister_id, wasm_module, arg, sender)
    }

    /// Reinstall one existing canister with a new wasm module.
    pub fn reinstall_canister(
        &self,
        canister_id: Principal,
        wasm_module: Vec<u8>,
        arg: Vec<u8>,
        sender: Option<Principal>,
    ) -> Result<(), RejectResponse> {
        self.inner
            .reinstall_canister(canister_id, wasm_module, arg, sender)
    }

    /// Submit one raw update call without executing it immediately.
    pub fn submit_call(
        &self,
        canister_id: Principal,
        sender: Principal,
        method: &str,
        payload: Vec<u8>,
    ) -> Result<RawMessageId, RejectResponse> {
        self.inner.submit_call(canister_id, sender, method, payload)
    }

    /// Await one previously submitted raw update call.
    pub fn await_call(&self, message_id: RawMessageId) -> Result<Vec<u8>, RejectResponse> {
        self.inner.await_call(message_id)
    }

    /// Fetch one canister status snapshot from PocketIC.
    pub fn canister_status(
        &self,
        canister_id: Principal,
        sender: Option<Principal>,
    ) -> Result<CanisterStatusResult, RejectResponse> {
        self.inner.canister_status(canister_id, sender)
    }

    /// Fetch one canister log stream from PocketIC.
    pub fn fetch_canister_logs(
        &self,
        canister_id: Principal,
        sender: Principal,
    ) -> Result<Vec<pocket_ic::CanisterLogRecord>, RejectResponse> {
        self.inner.fetch_canister_logs(canister_id, sender)
    }

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
