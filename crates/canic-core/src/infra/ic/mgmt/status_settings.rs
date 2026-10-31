//! Module: infra::ic::mgmt::status_settings
//!
//! Responsibility: perform raw Canister-history, status and settings management calls.
//! Does not own: status policy, deployment orchestration, or public DTO shaping.
//! Boundary: extends `MgmtInfra` with status and settings effects.

use super::MgmtInfra;
use crate::infra::ic::{IcInfraError, external_candid::decode_args};
use ic_call::{Call, WaitPolicy};
use ic_platform::candid::contract::encode_one;
use ic_platform::{
    management_canister_types::{
        CanisterIdRecord, CanisterInfoArgs, CanisterInfoResult, CanisterStatusResult,
        UpdateSettingsArgs,
    },
    principal::ExternalPrincipal as Principal,
};

impl MgmtInfra {
    /// Quote the same single-entry replicated history read used for effect recovery.
    pub fn canister_history_call_cost(canister_pid: Principal) -> Result<u128, IcInfraError> {
        Ok(Call::new(
            Principal::management_canister().into(),
            "canister_info",
            WaitPolicy::Bounded {
                timeout_seconds: 300,
            },
        )
        .take_raw_args(encode_one(CanisterInfoArgs {
            canister_id: canister_pid,
            num_requested_changes: Some(1),
        })?)
        .get_cost())
    }

    /// Observe the latest history entry through a replicated management call.
    pub async fn canister_history(
        canister_pid: Principal,
    ) -> Result<ic_call::Response, IcInfraError> {
        Call::new(
            Principal::management_canister().into(),
            "canister_info",
            WaitPolicy::Bounded {
                timeout_seconds: 300,
            },
        )
        .take_raw_args(encode_one(CanisterInfoArgs {
            canister_id: canister_pid,
            num_requested_changes: Some(1),
        })?)
        .execute()
        .await
        .map_err(IcInfraError::from)
    }

    /// Read one canister's monotonic management-history change count.
    pub async fn canister_history_total_changes(
        canister_pid: Principal,
    ) -> Result<u64, IcInfraError> {
        let args = CanisterInfoArgs {
            canister_id: canister_pid,
            num_requested_changes: None,
        };
        let response = Call::new(
            Principal::management_canister().into(),
            "canister_info",
            WaitPolicy::Bounded {
                timeout_seconds: 300,
            },
        )
        .take_raw_args(encode_one(args)?)
        .execute()
        .await?;
        let (info,): (CanisterInfoResult,) = decode_args(&response.into_bytes())?;
        Ok(info.total_num_changes)
    }

    /// Query the management canister for a canister's status.
    pub async fn canister_status(
        canister_pid: Principal,
    ) -> Result<CanisterStatusResult, IcInfraError> {
        let response = canister_status_call(canister_pid)?.execute().await?;
        let (status,): (CanisterStatusResult,) = decode_args(&response.into_bytes())?;

        Ok(status)
    }

    /// Quote the same encoded status call without scheduling an outbound effect.
    pub fn canister_status_call_cost(canister_pid: Principal) -> Result<u128, IcInfraError> {
        Ok(canister_status_call(canister_pid)?.get_cost())
    }

    /// Quote the same controller-update request before retaining its paid intent.
    pub fn update_settings_call_cost(args: &UpdateSettingsArgs) -> Result<u128, IcInfraError> {
        Ok(Call::new(
            Principal::management_canister().into(),
            "update_settings",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(args.clone())?)
        .get_cost())
    }

    /// Update canister settings through the management canister.
    pub async fn update_settings(args: &UpdateSettingsArgs) -> Result<(), IcInfraError> {
        Call::new(
            Principal::management_canister().into(),
            "update_settings",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(args.clone())?)
        .execute()
        .await?;

        Ok(())
    }
}

fn canister_status_call(canister_pid: Principal) -> Result<Call<'static, 'static>, IcInfraError> {
    Ok(Call::new(
        Principal::management_canister().into(),
        "canister_status",
        WaitPolicy::Bounded {
            timeout_seconds: 300,
        },
    )
    .take_raw_args(encode_one(CanisterIdRecord {
        canister_id: canister_pid,
    })?))
}
