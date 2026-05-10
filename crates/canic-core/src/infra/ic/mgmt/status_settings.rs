use crate::{
    cdk::candid::Principal,
    infra::{InfraError, ic::call::Call},
};

use super::{
    MgmtInfra,
    types::{InfraCanisterIdRecord, InfraCanisterStatusResult, InfraUpdateSettingsArgs},
};

impl MgmtInfra {
    // Query the management canister for a canister's status.
    pub async fn canister_status(
        canister_pid: Principal,
    ) -> Result<InfraCanisterStatusResult, InfraError> {
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

    // Updates canister settings via the management canister.
    pub async fn update_settings(args: &InfraUpdateSettingsArgs) -> Result<(), InfraError> {
        Call::bounded_wait(Principal::management_canister(), "update_settings")
            .with_arg(args.clone())?
            .execute()
            .await?;

        Ok(())
    }
}
