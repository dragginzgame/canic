//! Module: workflow::ic::icp_refill::manual
//!
//! Responsibility: validate and execute manual ICP refill requests.
//! Does not own: ledger execution, replay storage schema, or endpoint authorization.
//! Boundary: coordinates manual request preflight, replay reservation, and execution.

use crate::{
    InternalError,
    dto::icp_refill::{IcpRefillDryRun, IcpRefillRequest, IcpRefillResponse},
    ops::ic::IcOps,
    workflow::ic::icp_refill::{
        IcpRefillWorkflow, IcpRefillWorkflowError, RateQueryMode, estimate_cycles,
        execution::execute_fresh_manual_refill,
        prepare_context,
        replay::{
            IcpRefillReplayReservation, icp_refill_replay_reserve_input,
            log_icp_refill_committed_replay, log_icp_refill_fresh_reservation,
            reserve_icp_refill_replay,
        },
        require_icp_refill_configured,
    },
};

impl IcpRefillWorkflow {
    pub async fn dry_run_manual_refill(
        request: IcpRefillRequest,
    ) -> Result<IcpRefillDryRun, InternalError> {
        validate_manual_request_shape(&request, true)?;
        require_icp_refill_configured()?;
        let root_canister = IcOps::canister_self();
        let context = prepare_context(&request, root_canister, RateQueryMode::Always).await?;

        Ok(IcpRefillDryRun {
            operation_id: request.operation_id,
            amount_e8s: request.amount_e8s,
            fee_e8s: context.fee_e8s,
            xdr_permyriad_per_icp: context.xdr_permyriad_per_icp,
            estimated_cycles: context
                .xdr_permyriad_per_icp
                .map(|rate| estimate_cycles(request.amount_e8s, rate)),
        })
    }

    pub async fn execute_manual_refill(
        request: IcpRefillRequest,
    ) -> Result<IcpRefillResponse, InternalError> {
        validate_manual_request_shape(&request, false)?;
        require_icp_refill_configured()?;
        let root_canister = IcOps::canister_self();
        let replay_input = icp_refill_replay_reserve_input(
            &request,
            IcOps::msg_caller(),
            root_canister,
            IcOps::now_nanos(),
        );
        let reservation = reserve_icp_refill_replay(replay_input)?;

        match reservation {
            IcpRefillReplayReservation::Fresh {
                operation_id,
                token,
            } => {
                log_icp_refill_fresh_reservation(&request, root_canister);
                execute_fresh_manual_refill(request, operation_id, root_canister, &token).await
            }
            IcpRefillReplayReservation::Replay(response) => {
                log_icp_refill_committed_replay(&response);
                Ok(response)
            }
        }
    }
}

fn validate_manual_request_shape(
    request: &IcpRefillRequest,
    allow_dry_run: bool,
) -> Result<(), InternalError> {
    if request.dry_run && !allow_dry_run {
        return Err(IcpRefillWorkflowError::DryRunRequest.into());
    }
    Ok(())
}
