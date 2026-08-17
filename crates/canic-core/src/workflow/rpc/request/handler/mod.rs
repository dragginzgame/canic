//! Module: workflow::rpc::request::handler
//!
//! Responsibility: orchestrate root-bound RPC request replay, authorization, and execution.
//! Does not own: endpoint authentication, pure policy decisions, or storage record schema.
//! Boundary: calls ops and helper workflow modules after request DTOs are mapped.

#[cfg(test)]
mod tests;

mod authorize;
pub(in crate::workflow::rpc) mod capability;
mod execute;
mod nonroot_cycles;
mod replay;

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::rpc::Response,
    log,
    log::Topic,
    ops::{
        ic::IcOps,
        replay::guard::ReplayPending,
        runtime::{
            env::EnvOps,
            metrics::root_capability::{RootCapabilityMetricOutcome, RootCapabilityMetrics},
        },
    },
    workflow::rpc::{RootCapabilityAuthority, RootCapabilityLifecycleExecutor},
};

use capability::{RootCapability, RootReplayInput};

pub(in crate::workflow::rpc) use nonroot_cycles::NonrootCyclesCapabilityWorkflow;

const REPLAY_PURGE_SCAN_LIMIT: usize = 256;
const MAX_ROOT_REPLAY_ENTRIES: usize = 10_000;
const MAX_ROOT_REPLAY_ENTRIES_PER_CALLER: usize = 512;
const MAX_ROOT_TTL_NS: u64 = 300_000_000_000;
const REPLAY_PAYLOAD_HASH_DOMAIN: &[u8] = b"root-replay-payload-hash:v1";

///
/// RootContext
///
/// Runtime context extracted once for root RPC request handling.
///

#[derive(Clone, Copy, Debug)]
struct RootContext {
    caller: Principal,
    self_pid: Principal,
    is_root_env: bool,
    subnet_id: Principal,
    now: u64,
}

///
/// PreparedExecution
///
/// Replay reservation for an authorized capability execution.
///

#[derive(Clone, Debug)]
struct PreparedExecution {
    pending: ReplayPending,
}

///
/// RootPreflight
///
/// Result of replay and authorization checks before capability execution.
///

#[derive(Debug)]
enum RootPreflight {
    Fresh(PreparedExecution),
    Cached(Response),
}

///
/// RootResponseWorkflow
///
/// Workflow entry point for root-bound request execution.
///

pub(in crate::workflow::rpc) struct RootResponseWorkflow;

impl RootResponseWorkflow {
    /// Handle a capability already mapped by the envelope workflow.
    pub(in crate::workflow::rpc) async fn response_capability_replay_first(
        capability: RootCapability,
        authority: &RootCapabilityAuthority,
        lifecycle: &dyn RootCapabilityLifecycleExecutor,
    ) -> Result<Response, InternalError> {
        if let RootCapability::RequestCycles(req) = capability {
            let response = nonroot_cycles::response_replay_first_root(req, authority).await?;
            return Ok(Response::Cycles(response));
        }
        if matches!(capability, RootCapability::AcknowledgePlacementReceipt(_)) {
            return Self::response_idempotent(capability, authority);
        }

        Self::response(capability, authority, lifecycle).await
    }

    fn response_idempotent(
        capability: RootCapability,
        authority: &RootCapabilityAuthority,
    ) -> Result<Response, InternalError> {
        let ctx = Self::extract_root_context()?;
        crate::perf!("extract_context");
        let descriptor = capability.descriptor();
        crate::perf!("map_request");
        Self::authorize(&ctx, &capability, authority)?;
        crate::perf!("authorize");

        let RootCapability::AcknowledgePlacementReceipt(req) = capability else {
            unreachable!("only receipt acknowledgement is response-idempotent")
        };
        let result = execute::execute_placement_receipt_acknowledgement(&ctx, &req);
        crate::perf!("execute_capability");
        match result {
            Ok(response) => {
                RootCapabilityMetrics::record_execution(
                    descriptor.key,
                    RootCapabilityMetricOutcome::Success,
                );
                Ok(response)
            }
            Err(err) => {
                log!(
                    Topic::Rpc,
                    Warn,
                    "execute response-idempotent capability failed (capability={}, caller={}, subnet={}, now={}): {err}",
                    descriptor.name,
                    ctx.caller,
                    ctx.subnet_id,
                    ctx.now
                );
                RootCapabilityMetrics::record_execution(
                    descriptor.key,
                    RootCapabilityMetricOutcome::Error,
                );
                Err(err)
            }
        }
    }

    async fn response(
        capability: RootCapability,
        authority: &RootCapabilityAuthority,
        lifecycle: &dyn RootCapabilityLifecycleExecutor,
    ) -> Result<Response, InternalError> {
        let ctx = Self::extract_root_context()?;
        crate::perf!("extract_context");
        let descriptor = capability.descriptor();
        crate::perf!("map_request");

        let preflight = Self::preflight(&ctx, &capability, authority)?;
        crate::perf!("preflight");
        let prepared = match preflight {
            RootPreflight::Fresh(prepared) => prepared,
            RootPreflight::Cached(response) => return Ok(response),
        };

        let response = match Self::execute_root_capability(
            &ctx,
            &prepared.pending,
            capability,
            authority,
            lifecycle,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => {
                let err = Self::abort_replay_after_failure(prepared.pending, err);
                RootCapabilityMetrics::record_execution(
                    descriptor.key,
                    RootCapabilityMetricOutcome::Error,
                );
                return Err(err);
            }
        };
        crate::perf!("execute_capability");
        if let Err(err) = Self::commit_replay(&prepared.pending) {
            if let Err(_recovery_err) = Self::mark_replay_recovery_required(
                &prepared.pending,
                crate::model::replay::RecoveryReason::ResponseCommitFailed,
            ) {
                return Err(err);
            }
            log!(
                Topic::Rpc,
                Warn,
                "replay finalize failed after successful capability execution (capability={}, caller={}, subnet={}, now={}): {err}",
                descriptor.name,
                ctx.caller,
                ctx.subnet_id,
                ctx.now
            );
            RootCapabilityMetrics::record_execution(
                descriptor.key,
                RootCapabilityMetricOutcome::Error,
            );
            return Err(err);
        }
        crate::perf!("commit_replay");
        RootCapabilityMetrics::record_execution(
            descriptor.key,
            RootCapabilityMetricOutcome::Success,
        );

        Ok(response)
    }

    fn preflight(
        ctx: &RootContext,
        capability: &RootCapability,
        authority: &RootCapabilityAuthority,
    ) -> Result<RootPreflight, InternalError> {
        match Self::check_replay(ctx, capability)? {
            replay::ReplayPreflight::Fresh(pending) => {
                if let Err(err) = Self::authorize(ctx, capability, authority) {
                    return Err(Self::abort_replay_after_failure(pending, err));
                }
                Ok(RootPreflight::Fresh(PreparedExecution { pending }))
            }
            replay::ReplayPreflight::Cached(response) => Ok(RootPreflight::Cached(response)),
        }
    }

    fn authorize(
        ctx: &RootContext,
        capability: &RootCapability,
        authority: &RootCapabilityAuthority,
    ) -> Result<(), InternalError> {
        authorize::authorize(ctx, capability, authority)
    }

    async fn execute_root_capability(
        ctx: &RootContext,
        pending: &ReplayPending,
        capability: RootCapability,
        authority: &RootCapabilityAuthority,
        lifecycle: &dyn RootCapabilityLifecycleExecutor,
    ) -> Result<Response, InternalError> {
        execute::execute_root_capability(ctx, pending, capability, authority, lifecycle).await
    }

    fn check_replay(
        ctx: &RootContext,
        capability: &RootCapability,
    ) -> Result<replay::ReplayPreflight, InternalError> {
        replay::check_replay(ctx, capability)
    }

    fn commit_replay(pending: &ReplayPending) -> Result<(), InternalError> {
        replay::commit_replay(pending)
    }

    fn abort_replay_after_failure(pending: ReplayPending, error: InternalError) -> InternalError {
        replay::abort_replay_after_failure(pending, error)
    }

    fn mark_replay_recovery_required(
        pending: &ReplayPending,
        reason: crate::model::replay::RecoveryReason,
    ) -> Result<(), InternalError> {
        replay::mark_recovery_required(pending, reason)
    }

    fn extract_root_context() -> Result<RootContext, InternalError> {
        Ok(RootContext {
            caller: IcOps::msg_caller(),
            self_pid: IcOps::canister_self(),
            is_root_env: EnvOps::is_root(),
            subnet_id: EnvOps::subnet_pid()?,
            now: IcOps::now_secs(),
        })
    }
}
