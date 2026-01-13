use candid::{CandidType, Principal, decode_one, encode_args, encode_one, utils::ArgumentEncoder};
use canic::{
    Error,
    cdk::types::TC,
    dto::{
        abi::v1::CanisterInitPayload,
        env::EnvView,
        subnet::SubnetIdentity,
        topology::{AppDirectoryView, SubnetDirectoryView},
    },
    ids::CanisterRole,
};
use derive_more::{Deref, DerefMut};
use pocket_ic::{PocketIc, PocketIcBuilder};
use serde::de::DeserializeOwned;

const INSTALL_CYCLES: u128 = 500 * TC;

///
/// Create a fresh PocketIC universe.
///
/// IMPORTANT:
/// - Each call creates a new IC instance
/// - This must NOT be cached or shared across tests
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
    ) -> Result<Principal, Error> {
        let init_bytes = install_args_with_directories(role, app_directory, subnet_directory)?;

        Ok(self.create_funded_and_install(wasm, init_bytes))
    }

    fn create_funded_and_install(&self, wasm: Vec<u8>, init_bytes: Vec<u8>) -> Principal {
        let canister_id = self.create_canister();
        self.add_cycles(canister_id, INSTALL_CYCLES);
        self.0.install_canister(canister_id, wasm, init_bytes, None);

        canister_id
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
            .0
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
            .0
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
            .0
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
            .0
            .query_call(canister_id, caller, method, bytes)
            .map_err(|err| {
                Error::internal(format!(
                    "pocket_ic query_call failed (canister={canister_id}, method={method}): {err}"
                ))
            })?;

        decode_one(&result).map_err(|err| Error::internal(format!("decode_one failed: {err}")))
    }

    pub fn tick_n(&self, times: usize) {
        for _ in 0..times {
            self.tick();
        }
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
fn install_args(role: CanisterRole) -> Result<Vec<u8>, Error> {
    if role.is_root() {
        install_root_args()
    } else {
        // Non-root standalone install.
        // Provide only what is structurally known at install time.
        let env = EnvView {
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
            app_directory: AppDirectoryView(Vec::new()),
            subnet_directory: SubnetDirectoryView(Vec::new()),
        };

        encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
            .map_err(|err| Error::internal(format!("encode_args failed: {err}")))
    }
}

fn install_root_args() -> Result<Vec<u8>, Error> {
    encode_one(SubnetIdentity::Manual)
        .map_err(|err| Error::internal(format!("encode_one failed: {err}")))
}

fn install_args_with_directories(
    role: CanisterRole,
    app_directory: AppDirectoryView,
    subnet_directory: SubnetDirectoryView,
) -> Result<Vec<u8>, Error> {
    if role.is_root() {
        // Root canister: runtime identity only.
        // No fake principals. Runtime/bootstrap will resolve actual context.
        encode_one(SubnetIdentity::Manual)
            .map_err(|err| Error::internal(format!("encode_one failed: {err}")))
    } else {
        // Non-root canister: pass structural context, not invented identities.
        let env = EnvView {
            prime_root_pid: None,
            subnet_role: None,
            subnet_pid: None,
            root_pid: None,
            canister_role: Some(role),
            parent_pid: None,
        };

        let payload = CanisterInitPayload {
            env,
            app_directory,
            subnet_directory,
        };

        encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
            .map_err(|err| Error::internal(format!("encode_args failed: {err}")))
    }
}
