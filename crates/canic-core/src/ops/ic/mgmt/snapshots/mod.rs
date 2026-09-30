//! Module: ops::ic::mgmt::snapshots
//!
//! Responsibility: observe management snapshot calls and preserve typed failures.
//! Does not own: reset authority, retention policy or lifecycle orchestration.
//! Boundary: one management effect per operation, delegated to infra.

use super::{MgmtInfra, MgmtOps, management_call};
use crate::{InternalError, cdk::types::Principal};

impl MgmtOps {
    /// List exact snapshot identities before a caller-authorized cleanup.
    pub async fn list_canister_snapshot_ids(
        canister_id: Principal,
    ) -> Result<Vec<Vec<u8>>, InternalError> {
        management_call(MgmtInfra::list_canister_snapshot_ids(canister_id)).await
    }

    /// Delete one exact observed snapshot and retain management-call metrics.
    pub async fn delete_canister_snapshot(
        canister_id: Principal,
        snapshot_id: Vec<u8>,
    ) -> Result<(), InternalError> {
        management_call(MgmtInfra::delete_canister_snapshot(
            canister_id,
            snapshot_id,
        ))
        .await
    }
}
