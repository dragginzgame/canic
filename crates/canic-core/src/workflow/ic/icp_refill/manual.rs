use super::{
    IcpRefillWorkflow, IcpRefillWorkflowError, RateQueryMode, estimate_cycles,
    execution::execute_fresh_manual_refill,
    prepare_context,
    replay::{
        IcpRefillReplayReservation, icp_refill_replay_reserve_input,
        log_icp_refill_committed_replay, log_icp_refill_fresh_reservation,
        reserve_icp_refill_replay,
    },
};
use crate::{
    InternalError,
    dto::icp_refill::{IcpRefillDryRun, IcpRefillMode, IcpRefillRequest, IcpRefillResponse},
    ops::ic::IcOps,
};

impl IcpRefillWorkflow {
    pub async fn dry_run_manual_refill(
        request: IcpRefillRequest,
    ) -> Result<IcpRefillDryRun, InternalError> {
        validate_manual_request_shape(&request, true)?;
        let context = prepare_context(&request, RateQueryMode::Always).await?;

        Ok(IcpRefillDryRun {
            operation_id: request.operation_id,
            mode: request.mode,
            amount_e8s: request.amount_e8s,
            fee_e8s: context.fee_e8s,
            xdr_permyriad_per_icp: context.xdr_permyriad_per_icp,
            estimated_cycles: context
                .xdr_permyriad_per_icp
                .map(|rate| estimate_cycles(request.amount_e8s, rate)),
            message: dry_run_message(request.mode),
        })
    }

    pub async fn execute_manual_refill(
        request: IcpRefillRequest,
    ) -> Result<IcpRefillResponse, InternalError> {
        validate_manual_request_shape(&request, false)?;
        let replay_input =
            icp_refill_replay_reserve_input(&request, IcOps::msg_caller(), IcOps::now_nanos());
        let reservation = reserve_icp_refill_replay(replay_input)?;

        match reservation {
            IcpRefillReplayReservation::Fresh {
                operation_id,
                token,
            } => {
                log_icp_refill_fresh_reservation(&request);
                execute_fresh_manual_refill(request, operation_id, &token).await
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
    if request.mode != IcpRefillMode::Canister {
        return Err(IcpRefillWorkflowError::UnsupportedMode.into());
    }
    if request.dry_run && !allow_dry_run {
        return Err(IcpRefillWorkflowError::DryRunRequest.into());
    }
    let self_pid = IcOps::canister_self();
    if request.source_canister != self_pid {
        return Err(IcpRefillWorkflowError::SourceCanisterMismatch {
            source_canister: request.source_canister,
            self_pid,
        }
        .into());
    }

    Ok(())
}

pub(super) fn dry_run_message(mode: IcpRefillMode) -> Option<String> {
    match mode {
        IcpRefillMode::Canister => None,
        IcpRefillMode::Fabricate => {
            Some("mode=fabricate (does not call canister refill endpoint)".to_string())
        }
    }
}
