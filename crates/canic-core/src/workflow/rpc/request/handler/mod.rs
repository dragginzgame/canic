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
/// Replay reservation plus any authorization artifact required for execution.
///

#[derive(Clone, Debug)]
struct PreparedExecution {
    pending: ReplayPending,
    authorized_cycles: Option<nonroot_cycles::AuthorizedCyclesGrant>,
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
    ) -> Result<Response, InternalError> {
        if let RootCapability::RequestCycles(req) = capability {
            let response = nonroot_cycles::response_replay_first_root(req).await?;
            return Ok(Response::Cycles(response));
        }

        Self::response(capability).await
    }

    async fn response(capability: RootCapability) -> Result<Response, InternalError> {
        let ctx = Self::extract_root_context()?;
        crate::perf!("extract_context");
        let descriptor = capability.descriptor();
        crate::perf!("map_request");

        let preflight = Self::preflight(&ctx, &capability)?;
        crate::perf!("preflight");
        let prepared = match preflight {
            RootPreflight::Fresh(prepared) => prepared,
            RootPreflight::Cached(response) => return Ok(response),
        };

        let response = match Self::execute_root_capability(
            &ctx,
            &prepared.pending,
            capability,
            prepared.authorized_cycles,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => {
                Self::abort_replay(prepared.pending)?;
                RootCapabilityMetrics::record_execution(
                    descriptor.key,
                    RootCapabilityMetricOutcome::Error,
                );
                return Err(err);
            }
        };
        crate::perf!("execute_capability");
        if let Err(err) = Self::commit_replay(&prepared.pending, &response) {
            Self::mark_replay_recovery_required(
                &prepared.pending,
                crate::model::replay::RecoveryReason::ResponseCommitFailed,
            )?;
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
    ) -> Result<RootPreflight, InternalError> {
        match Self::check_replay(ctx, capability)? {
            replay::ReplayPreflight::Fresh(pending) => {
                let authorized_cycles = match Self::authorize_with_hint(ctx, capability) {
                    Ok(authorized_cycles) => authorized_cycles,
                    Err(err) => {
                        Self::abort_replay(pending)?;
                        return Err(err);
                    }
                };
                Ok(RootPreflight::Fresh(PreparedExecution {
                    pending,
                    authorized_cycles,
                }))
            }
            replay::ReplayPreflight::Cached(response) => Ok(RootPreflight::Cached(response)),
        }
    }

    fn authorize_with_hint(
        ctx: &RootContext,
        capability: &RootCapability,
    ) -> Result<Option<nonroot_cycles::AuthorizedCyclesGrant>, InternalError> {
        if let RootCapability::RequestCycles(req) = capability {
            return if ctx.is_root_env {
                nonroot_cycles::authorize_root_request_cycles_plan(ctx, req).map(Some)
            } else {
                nonroot_cycles::authorize_request_cycles_plan(ctx, req).map(Some)
            };
        }

        Self::authorize(ctx, capability)?;
        Ok(None)
    }

    fn authorize(ctx: &RootContext, capability: &RootCapability) -> Result<(), InternalError> {
        authorize::authorize(ctx, capability)
    }

    async fn execute_root_capability(
        ctx: &RootContext,
        pending: &ReplayPending,
        capability: RootCapability,
        authorized_cycles: Option<nonroot_cycles::AuthorizedCyclesGrant>,
    ) -> Result<Response, InternalError> {
        execute::execute_root_capability(ctx, pending, capability, authorized_cycles).await
    }

    fn check_replay(
        ctx: &RootContext,
        capability: &RootCapability,
    ) -> Result<replay::ReplayPreflight, InternalError> {
        replay::check_replay(ctx, capability)
    }

    fn commit_replay(pending: &ReplayPending, response: &Response) -> Result<(), InternalError> {
        replay::commit_replay(pending, response)
    }

    fn abort_replay(pending: ReplayPending) -> Result<(), InternalError> {
        replay::abort_replay(pending)
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

#[cfg(test)]
fn hash_domain_separated(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    replay::hash_domain_separated(domain, payload)
}
