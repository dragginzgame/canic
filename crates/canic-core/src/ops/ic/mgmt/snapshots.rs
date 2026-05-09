use super::*;

impl MgmtOps {
    /// Create a canister snapshot via the management canister and record metrics.
    pub async fn take_canister_snapshot(
        canister_pid: Principal,
        replace_snapshot: Option<Vec<u8>>,
        uninstall_code: Option<bool>,
    ) -> Result<CanisterSnapshot, InternalError> {
        record_unscoped_canister_op(
            CanisterOpsMetricOperation::Snapshot,
            CanisterOpsMetricOutcome::Started,
            CanisterOpsMetricReason::Ok,
        );

        match management_call(
            ManagementCallMetricOperation::TakeCanisterSnapshot,
            MgmtInfra::take_canister_snapshot(canister_pid, replace_snapshot, uninstall_code),
        )
        .await
        {
            Ok(snapshot) => {
                record_unscoped_canister_op(
                    CanisterOpsMetricOperation::Snapshot,
                    CanisterOpsMetricOutcome::Completed,
                    CanisterOpsMetricReason::Ok,
                );
                log!(
                    Topic::CanisterLifecycle,
                    Ok,
                    "take_canister_snapshot: {canister_pid} total_size={}",
                    snapshot.total_size
                );
                Ok(canister_snapshot_from_infra(snapshot))
            }
            Err(err) => {
                record_unscoped_canister_op(
                    CanisterOpsMetricOperation::Snapshot,
                    CanisterOpsMetricOutcome::Failed,
                    CanisterOpsMetricReason::from_error(&err),
                );
                Err(err)
            }
        }
    }

    /// Restore a canister from a snapshot via the management canister and record metrics.
    pub async fn load_canister_snapshot(
        canister_pid: Principal,
        snapshot_id: Vec<u8>,
    ) -> Result<(), InternalError> {
        record_unscoped_canister_op(
            CanisterOpsMetricOperation::Restore,
            CanisterOpsMetricOutcome::Started,
            CanisterOpsMetricReason::Ok,
        );

        match management_call(
            ManagementCallMetricOperation::LoadCanisterSnapshot,
            MgmtInfra::load_canister_snapshot(canister_pid, snapshot_id),
        )
        .await
        {
            Ok(()) => {
                record_unscoped_canister_op(
                    CanisterOpsMetricOperation::Restore,
                    CanisterOpsMetricOutcome::Completed,
                    CanisterOpsMetricReason::Ok,
                );
                log!(
                    Topic::CanisterLifecycle,
                    Ok,
                    "load_canister_snapshot: {canister_pid}"
                );
                Ok(())
            }
            Err(err) => {
                record_unscoped_canister_op(
                    CanisterOpsMetricOperation::Restore,
                    CanisterOpsMetricOutcome::Failed,
                    CanisterOpsMetricReason::from_error(&err),
                );
                Err(err)
            }
        }
    }
}
