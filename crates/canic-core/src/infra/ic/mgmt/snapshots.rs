use crate::{
    cdk::{api, candid::Principal},
    infra::{InfraError, ic::call::Call},
};

use super::{
    MgmtInfra,
    types::{InfraCanisterSnapshot, InfraLoadCanisterSnapshotArgs, InfraTakeCanisterSnapshotArgs},
};

impl MgmtInfra {
    // Creates one canister snapshot through the management canister.
    pub async fn take_canister_snapshot(
        canister_pid: Principal,
        replace_snapshot: Option<Vec<u8>>,
        uninstall_code: Option<bool>,
    ) -> Result<InfraCanisterSnapshot, InfraError> {
        let args = InfraTakeCanisterSnapshotArgs {
            canister_id: canister_pid,
            replace_snapshot,
            uninstall_code,
            sender_canister_version: Some(api::canister_version()),
        };
        let response =
            Call::bounded_wait(Principal::management_canister(), "take_canister_snapshot")
                .with_arg(args)?
                .execute()
                .await?;
        let (snapshot,): (InfraCanisterSnapshot,) = response.candid_tuple()?;

        Ok(snapshot)
    }

    // Loads one canister snapshot through the management canister.
    pub async fn load_canister_snapshot(
        canister_pid: Principal,
        snapshot_id: Vec<u8>,
    ) -> Result<(), InfraError> {
        let args = InfraLoadCanisterSnapshotArgs {
            canister_id: canister_pid,
            snapshot_id,
            sender_canister_version: Some(api::canister_version()),
        };
        Call::bounded_wait(Principal::management_canister(), "load_canister_snapshot")
            .with_arg(args)?
            .execute()
            .await?;

        Ok(())
    }
}
