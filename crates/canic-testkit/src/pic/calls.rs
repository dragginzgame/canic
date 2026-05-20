use candid::{CandidType, Principal, decode_one, encode_args, utils::ArgumentEncoder};
use serde::de::DeserializeOwned;

use super::{Pic, PicCallError};

impl Pic {
    /// Generic update call helper (serializes args + decodes result).
    pub fn update_call<T, A>(
        &self,
        canister_id: Principal,
        method: &str,
        args: A,
    ) -> Result<T, PicCallError>
    where
        T: CandidType + DeserializeOwned,
        A: ArgumentEncoder,
    {
        let bytes: Vec<u8> = encode_args(args)
            .map_err(|err| PicCallError::new(format!("encode_args failed: {err}")))?;
        let result = self
            .inner
            .update_call(canister_id, Principal::anonymous(), method, bytes)
            .map_err(|err| {
                PicCallError::new(format!(
                    "pocket_ic update_call failed (canister={canister_id}, method={method}): {err}"
                ))
            })?;

        decode_one(&result).map_err(|err| PicCallError::new(format!("decode_one failed: {err}")))
    }

    /// Generic update call helper with an explicit caller principal.
    pub fn update_call_as<T, A>(
        &self,
        canister_id: Principal,
        caller: Principal,
        method: &str,
        args: A,
    ) -> Result<T, PicCallError>
    where
        T: CandidType + DeserializeOwned,
        A: ArgumentEncoder,
    {
        let bytes: Vec<u8> = encode_args(args)
            .map_err(|err| PicCallError::new(format!("encode_args failed: {err}")))?;
        let result = self
            .inner
            .update_call(canister_id, caller, method, bytes)
            .map_err(|err| {
                PicCallError::new(format!(
                    "pocket_ic update_call failed (canister={canister_id}, method={method}): {err}"
                ))
            })?;

        decode_one(&result).map_err(|err| PicCallError::new(format!("decode_one failed: {err}")))
    }

    /// Generic query call helper.
    pub fn query_call<T, A>(
        &self,
        canister_id: Principal,
        method: &str,
        args: A,
    ) -> Result<T, PicCallError>
    where
        T: CandidType + DeserializeOwned,
        A: ArgumentEncoder,
    {
        let bytes: Vec<u8> = encode_args(args)
            .map_err(|err| PicCallError::new(format!("encode_args failed: {err}")))?;
        let result = self
            .inner
            .query_call(canister_id, Principal::anonymous(), method, bytes)
            .map_err(|err| {
                PicCallError::new(format!(
                    "pocket_ic query_call failed (canister={canister_id}, method={method}): {err}"
                ))
            })?;

        decode_one(&result).map_err(|err| PicCallError::new(format!("decode_one failed: {err}")))
    }

    /// Generic query call helper with an explicit caller principal.
    pub fn query_call_as<T, A>(
        &self,
        canister_id: Principal,
        caller: Principal,
        method: &str,
        args: A,
    ) -> Result<T, PicCallError>
    where
        T: CandidType + DeserializeOwned,
        A: ArgumentEncoder,
    {
        let bytes: Vec<u8> = encode_args(args)
            .map_err(|err| PicCallError::new(format!("encode_args failed: {err}")))?;
        let result = self
            .inner
            .query_call(canister_id, caller, method, bytes)
            .map_err(|err| {
                PicCallError::new(format!(
                    "pocket_ic query_call failed (canister={canister_id}, method={method}): {err}"
                ))
            })?;

        decode_one(&result).map_err(|err| PicCallError::new(format!("decode_one failed: {err}")))
    }

    /// Advance PocketIC by a fixed number of ticks.
    pub fn tick_n(&self, times: usize) {
        for _ in 0..times {
            self.tick();
        }
    }
}
