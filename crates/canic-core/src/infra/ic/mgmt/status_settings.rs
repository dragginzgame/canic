//! Module: infra::ic::mgmt::status_settings
//!
//! Responsibility: perform raw Canister-history, status and settings management calls.
//! Does not own: status policy, deployment orchestration, or public DTO shaping.
//! Boundary: extends `MgmtInfra` with status and settings effects.

use crate::{
    cdk::candid::Principal,
    infra::ic::{IcInfraError, call::Call},
};

use super::{
    MgmtInfra,
    types::{
        InfraCanisterIdRecord, InfraCanisterInfoArgs, InfraCanisterInfoResult,
        InfraCanisterStatusResult, InfraUpdateSettingsArgs,
    },
};

impl MgmtInfra {
    /// Read one canister's monotonic management-history change count.
    pub async fn canister_history_total_changes(
        canister_pid: Principal,
    ) -> Result<u64, IcInfraError> {
        let args = InfraCanisterInfoArgs {
            canister_id: canister_pid,
            num_requested_changes: None,
        };
        let response = Call::bounded_wait(Principal::management_canister(), "canister_info")
            .with_arg(args)?
            .execute()
            .await?;
        let (info,): (InfraCanisterInfoResult,) = response.candid_tuple()?;
        Ok(info.total_num_changes)
    }

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
