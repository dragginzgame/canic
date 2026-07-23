//! Module: infra::ic::mgmt::status_settings
//!
//! Responsibility: perform raw canister status and settings management calls.
//! Does not own: status policy, deployment orchestration, or public DTO shaping.
//! Boundary: extends `MgmtInfra` with status and settings effects.

use crate::{
    cdk::candid::Principal,
    infra::ic::{IcInfraError, call::Call},
};

use super::{
    MgmtInfra,
    types::{InfraCanisterIdRecord, InfraCanisterStatusResult, InfraUpdateSettingsArgs},
};

impl MgmtInfra {
    /// Query the management canister for a canister's status.
    pub async fn canister_status(
        canister_pid: Principal,
    ) -> Result<InfraCanisterStatusResult, IcInfraError> {
        let args = InfraCanisterIdRecord {
            canister_id: canister_pid,
        };
        let response = Call::bounded_wait(Principal::management_canister(), "canister_status")
            .with_arg(args)?
            .execute()
            .await?;
        let (status,): (InfraCanisterStatusResult,) = response.candid_tuple()?;

        Ok(status)
    }

    /// Update canister settings through the management canister.
    pub async fn update_settings(args: &InfraUpdateSettingsArgs) -> Result<(), IcInfraError> {
        Call::bounded_wait(Principal::management_canister(), "update_settings")
            .with_arg(args.clone())?
            .execute()
            .await?;

        Ok(())
    }
}
