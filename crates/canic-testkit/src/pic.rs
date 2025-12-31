use candid::{CandidType, Principal, decode_one, encode_args, encode_one, utils::ArgumentEncoder};
use canic::{
    PublicError,
    core::{
        dto::{
            abi::v1::CanisterInitPayload,
            directory::{AppDirectoryView, SubnetDirectoryView},
            env::EnvView,
            subnet::SubnetIdentity,
        },
        ids::{CanisterRole, SubnetRole},
    },
};
use derive_more::{Deref, DerefMut};
use pocket_ic::{PocketIc, PocketIcBuilder};
use serde::de::DeserializeOwned;
use std::sync::OnceLock;

///
/// PocketIC singleton
///
/// This crate models a *single* IC universe shared by all tests.
/// We intentionally reuse one `PocketIc` instance to preserve determinism and
/// to match the real IC's global, long-lived state.
///
/// Invariants:
/// - Exactly one `PocketIc` instance exists for the entire test run.
/// - All tests share the same universe (no resets between tests).
/// - Tests are single-threaded and must not assume isolation.
/// - Determinism is prioritized over per-test cleanliness.
///
/// The `OnceLock` is not about performance; it encodes these invariants so
/// tests cannot accidentally spin up extra universes.
///
static PIC: OnceLock<Pic> = OnceLock::new();

///
/// Access the singleton PocketIC wrapper.
///
/// The global instance is created on first use and then reused.
///
#[must_use]
pub fn pic() -> &'static Pic {
    PIC.get_or_init(|| PicBuilder::new().with_application_subnet().build())
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

#[allow(clippy::new_without_default)]
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
        Pic(self.0.build())
    }
}

///
/// Pic
/// Thin wrapper around the global PocketIC instance.
///
/// This type intentionally exposes only a minimal API surface; callers should
/// use `pic()` to obtain the singleton and then perform installs/calls.
///
#[derive(Deref, DerefMut)]
pub struct Pic(PocketIc);

impl Pic {
    /// Install a canister with the given type and wasm bytes.
    ///
    /// Install failures are treated as fatal in tests.
    pub fn create_and_install_canister(
        &self,
        role: CanisterRole,
        wasm: Vec<u8>,
    ) -> Result<Principal, PublicError> {
        // Create and fund the canister.
        let canister_id = self.create_canister();
        self.add_cycles(canister_id, 1_000_000_000_000);

        // Install with deterministic init arguments.
        let init_bytes = install_args(role)?;
        self.0.install_canister(canister_id, wasm, init_bytes, None);

        Ok(canister_id)
    }

    /// Install a canister with a custom directory snapshot (local-only helper).
    ///
    /// Use this when a test exercises directory-dependent auth/endpoints and
    /// cannot rely on root to provide a snapshot.
    pub fn create_and_install_canister_with_directories(
        &self,
        role: CanisterRole,
        wasm: Vec<u8>,
        app_directory: AppDirectoryView,
        subnet_directory: SubnetDirectoryView,
    ) -> Result<Principal, PublicError> {
        let canister_id = self.create_canister();
        self.add_cycles(canister_id, 1_000_000_000_000);

        let init_bytes = install_args_with_directories(role, app_directory, subnet_directory)?;
        self.0.install_canister(canister_id, wasm, init_bytes, None);

        Ok(canister_id)
    }

    /// Generic update call helper (serializes args + decodes result).
    pub fn update_call<T, A>(
        &self,
        canister_id: Principal,
        method: &str,
        args: A,
    ) -> Result<T, PublicError>
    where
        T: CandidType + DeserializeOwned,
        A: ArgumentEncoder,
    {
        let bytes: Vec<u8> = encode_args(args)?;
        let result = self
            .0
            .update_call(canister_id, Principal::anonymous(), method, bytes)
            .map_err(|_| PublicError {
                code: canic::core::ErrorCode::Internal,
                message: "test error".to_string(),
            })?;

        decode_one(&result).map_err(Into::into)
    }

    /// Generic query call helper.
    pub fn query_call<T, A>(
        &self,
        canister_id: Principal,
        method: &str,
        args: A,
    ) -> Result<T, PublicError>
    where
        T: CandidType + DeserializeOwned,
        A: ArgumentEncoder,
    {
        let bytes: Vec<u8> = encode_args(args)?;
        let result = self
            .0
            .query_call(canister_id, Principal::anonymous(), method, bytes)
            .map_err(|_| PublicError {
                code: canic::core::ErrorCode::Internal,
                message: "test error".to_string(),
            })?;

        decode_one(&result).map_err(Into::into)
    }
}

/// --------------------------------------
/// install_args helper
/// --------------------------------------
///
/// Init semantics:
/// - Root canisters receive a `SubnetIdentity` (direct root bootstrap).
/// - Non-root canisters receive `EnvView` + optional directory snapshots.
///
/// Directory handling:
/// - By default, directory views are empty for standalone installs.
/// - Directory-dependent logic is opt-in via `install_args_with_directories`.
/// - Root-provisioned installs will populate directories via cascade.
///
fn install_args(role: CanisterRole) -> Result<Vec<u8>, PublicError> {
    let args = if role.is_root() {
        // Provide a deterministic subnet principal for PocketIC runs.
        let subnet_pid = Principal::from_slice(&[0xAA; 29]);
        encode_one(SubnetIdentity::Manual(subnet_pid))
    } else {
        // Provide a minimal, deterministic env payload for standalone installs.
        let root_pid = Principal::from_slice(&[0xBB; 29]);
        let subnet_pid = Principal::from_slice(&[0xAA; 29]);
        let env = EnvView {
            prime_root_pid: Some(root_pid),
            subnet_role: Some(SubnetRole::PRIME),
            subnet_pid: Some(subnet_pid),
            root_pid: Some(root_pid),
            canister_role: Some(role),
            parent_pid: Some(root_pid),
        };

        // Intentional: local standalone installs don't need directory views unless a test
        // exercises directory-dependent auth/endpoints.
        let payload = CanisterInitPayload {
            env,
            app_directory: AppDirectoryView(Vec::new()),
            subnet_directory: SubnetDirectoryView(Vec::new()),
        };
        encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
    }?;

    Ok(args)
}

fn install_args_with_directories(
    role: CanisterRole,
    app_directory: AppDirectoryView,
    subnet_directory: SubnetDirectoryView,
) -> Result<Vec<u8>, PublicError> {
    let args = if role.is_root() {
        let subnet_pid = Principal::from_slice(&[0xAA; 29]);
        encode_one(SubnetIdentity::Manual(subnet_pid))
    } else {
        let root_pid = Principal::from_slice(&[0xBB; 29]);
        let subnet_pid = Principal::from_slice(&[0xAA; 29]);
        let env = EnvView {
            prime_root_pid: Some(root_pid),
            subnet_role: Some(SubnetRole::PRIME),
            subnet_pid: Some(subnet_pid),
            root_pid: Some(root_pid),
            canister_role: Some(role),
            parent_pid: Some(root_pid),
        };
        let payload = CanisterInitPayload {
            env,
            app_directory,
            subnet_directory,
        };
        encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
    }?;

    Ok(args)
}
