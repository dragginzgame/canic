use candid::{CandidType, decode_one, encode_args, encode_one, utils::ArgumentEncoder};
use canic::{
    Error,
    model::memory::topology::SubnetIdentity,
    ops::CanisterInitPayload,
    types::{CanisterType, Principal},
};
use derive_more::{Deref, DerefMut};
use pocket_ic::{PocketIc, PocketIcBuilder};
use serde::de::DeserializeOwned;

///
/// PicBuilder
///

pub struct PicBuilder(PocketIcBuilder);

#[allow(clippy::new_without_default)]
impl PicBuilder {
    /// Start a new PicBuilder with sensible defaults
    #[must_use]
    pub fn new() -> Self {
        Self(PocketIcBuilder::new())
    }

    #[must_use]
    pub fn with_application_subnet(mut self) -> Self {
        self.0 = self.0.with_application_subnet();
        self
    }

    #[must_use]
    pub fn with_nns_subnet(mut self) -> Self {
        self.0 = self.0.with_nns_subnet();
        self
    }

    /// Finish building the PocketIC instance and wrap it
    #[must_use]
    pub fn build(self) -> Pic {
        Pic(self.0.build())
    }
}

///
/// Pic
///

#[derive(Deref, DerefMut)]
pub struct Pic(PocketIc);

impl Pic {
    /// Install a canister with the given type and wasm bytes
    pub fn create_and_install_canister(
        &self,
        ty: CanisterType,
        wasm: Vec<u8>,
    ) -> Result<Principal, Error> {
        // Create and fund the canister
        let canister_id = self.create_canister();
        self.add_cycles(canister_id, 1_000_000_000_000);

        // Install
        let init_bytes = install_args(ty)?;
        self.0.install_canister(canister_id, wasm, init_bytes, None);

        Ok(canister_id)
    }

    /// Generic update call helper (serializes args + decodes result)
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
        let bytes: Vec<u8> = encode_args(args)?;
        let result = self
            .0
            .update_call(canister_id, Principal::anonymous(), method, bytes)
            .map_err(|e| Error::test(e.to_string()))?;

        decode_one(&result).map_err(Into::into)
    }

    /// Generic query call helper
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
        let bytes: Vec<u8> = encode_args(args)?;
        let result = self
            .0
            .query_call(canister_id, Principal::anonymous(), method, bytes)
            .map_err(|e| Error::test(e.to_string()))?;

        decode_one(&result).map_err(Into::into)
    }
}

/// --------------------------------------
/// install_args helper
/// --------------------------------------
fn install_args(ty: CanisterType) -> Result<Vec<u8>, Error> {
    let args = if ty.is_root() {
        encode_one(SubnetIdentity::Test)
    } else {
        let payload = CanisterInitPayload::empty();
        encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
    }?;

    Ok(args)
}
