use crate::{
    cdk::candid::Principal,
    infra::{InfraError, ic::IcInfraError, ic::call::Call},
};

use super::{MgmtInfra, MgmtInfraError};

impl MgmtInfra {
    // Query the management canister for raw randomness.
    pub async fn raw_rand() -> Result<[u8; 32], InfraError> {
        let response = Call::unbounded_wait(Principal::management_canister(), "raw_rand")
            .execute()
            .await?;

        let bytes: Vec<u8> = response.candid()?;
        let len = bytes.len();

        let seed: [u8; 32] = bytes
            .try_into()
            .map_err(|_| MgmtInfraError::RawRandInvalidLength { len })
            .map_err(IcInfraError::from)?;

        Ok(seed)
    }
}
