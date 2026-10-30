//! Module: infra::ic::mgmt::snapshots
//!
//! Responsibility: list and delete exact management-canister snapshots.
//! Does not own: reset authority, retention policy or lifecycle orchestration.
//! Boundary: raw management calls used by the observed operations facade.

use super::MgmtInfra;
use crate::infra::ic::external_candid::decode_args;
use crate::{cdk::types::Principal, infra::ic::IcInfraError};
use ic_call::{Call, WaitPolicy};
use ic_platform::candid::contract::encode_one;
use ic_platform::management_canister_types::{
    CanisterIdRecord, DeleteCanisterSnapshotArgs, Snapshot,
};

impl MgmtInfra {
    /// List exact snapshot identities retained by one canister.
    pub async fn list_canister_snapshot_ids(
        canister_id: Principal,
    ) -> Result<Vec<Vec<u8>>, IcInfraError> {
        let response = Call::new(
            Principal::management_canister().into(),
            "list_canister_snapshots",
            WaitPolicy::Bounded {
                timeout_seconds: 300,
            },
        )
        .take_raw_args(encode_one(CanisterIdRecord { canister_id })?)
        .execute()
        .await?;
        let (snapshots,): (Vec<Snapshot>,) = decode_args(&response.into_bytes())?;
        Ok(snapshots.into_iter().map(|snapshot| snapshot.id).collect())
    }

    /// Delete one observed snapshot without changing the installed canister.
    pub async fn delete_canister_snapshot(
        canister_id: Principal,
        snapshot_id: Vec<u8>,
    ) -> Result<(), IcInfraError> {
        Call::new(
            Principal::management_canister().into(),
            "delete_canister_snapshot",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(DeleteCanisterSnapshotArgs {
            canister_id,
            snapshot_id,
        })?)
        .execute()
        .await?;
        Ok(())
    }
}
