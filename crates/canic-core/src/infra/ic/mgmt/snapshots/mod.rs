//! Module: infra::ic::mgmt::snapshots
//!
//! Responsibility: list and delete exact management-canister snapshots.
//! Does not own: reset authority, retention policy or lifecycle orchestration.
//! Boundary: raw management calls used by the observed operations facade.

use super::{
    MgmtInfra,
    types::{InfraCanisterIdRecord, InfraCanisterSnapshot, InfraDeleteCanisterSnapshotArgs},
};
use crate::{
    cdk::types::Principal,
    infra::ic::{IcInfraError, call::Call},
};

impl MgmtInfra {
    /// List exact snapshot identities retained by one canister.
    pub async fn list_canister_snapshot_ids(
        canister_id: Principal,
    ) -> Result<Vec<Vec<u8>>, IcInfraError> {
        let response =
            Call::bounded_wait(Principal::management_canister(), "list_canister_snapshots")
                .with_arg(InfraCanisterIdRecord { canister_id })?
                .execute()
                .await?;
        let (snapshots,): (Vec<InfraCanisterSnapshot>,) = response.candid_tuple()?;
        Ok(snapshots.into_iter().map(|snapshot| snapshot.id).collect())
    }

    /// Delete one observed snapshot without changing the installed canister.
    pub async fn delete_canister_snapshot(
        canister_id: Principal,
        snapshot_id: Vec<u8>,
    ) -> Result<(), IcInfraError> {
        Call::unbounded_wait(Principal::management_canister(), "delete_canister_snapshot")
            .with_arg(InfraDeleteCanisterSnapshotArgs {
                canister_id,
                snapshot_id,
            })?
            .execute()
            .await?;
        Ok(())
    }
}
