#[cfg(test)]
use crate::dto::auth::RoleAttestation;
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::rpc::{Request, Response, RootCapabilityCommand},
    log,
    log::Topic,
    ops::{
        ic::IcOps,
        replay::guard::ReplayPending,
        runtime::env::EnvOps,
        runtime::metrics::root_capability::{RootCapabilityMetricOutcome, RootCapabilityMetrics},
    },
};

#[cfg(test)]
mod tests;

mod authorize;
mod capability;
mod delegation;
mod execute;
mod funding;
mod nonroot_cycles;
mod replay;
use capability::RootCapability;

pub use nonroot_cycles::NonrootCyclesCapabilityWorkflow;

const REPLAY_PURGE_SCAN_LIMIT: usize = 256;
const MAX_ROOT_REPLAY_ENTRIES: usize = 10_000;
const MAX_ROOT_TTL_SECONDS: u64 = 300;
const DEFAULT_MAX_ROLE_ATTESTATION_TTL_SECONDS: u64 = 900;
const REPLAY_PAYLOAD_HASH_DOMAIN: &[u8] = b"root-replay-payload-hash:v1";

///
/// RootContext
///

#[derive(Clone, Copy, Debug)]
struct RootContext {
    caller: Principal,
    self_pid: Principal,
    is_root_env: bool,
    subnet_id: Principal,
    now: u64,
}

#[derive(Clone, Copy, Debug)]
struct PreparedExecution {
    pending: ReplayPending,
    authorized_cycles: Option<nonroot_cycles::AuthorizedCyclesGrant>,
}

#[derive(Debug)]
enum RootPreflight {
    Fresh(PreparedExecution),
    Cached(Response),
}

///
/// AuthorizationPipelineOrder
///

#[derive(Clone, Copy, Debug)]
enum AuthorizationPipelineOrder {
    AuthorizeThenReplay,
    ReplayThenAuthorize,
}

///
/// RootResponseWorkflow
///

pub struct RootResponseWorkflow;

impl RootResponseWorkflow {
    /// Handle a root-bound orchestration request and produce a [`Response`].
    pub async fn response(req: Request) -> Result<Response, InternalError> {
        Self::response_with_pipeline(req, AuthorizationPipelineOrder::AuthorizeThenReplay).await
    }

    /// Handle a root-bound orchestration request using replay-before-policy
    /// ordering for capability-envelope execution.
    pub async fn response_replay_first(req: Request) -> Result<Response, InternalError> {
        Self::response_with_pipeline(req, AuthorizationPipelineOrder::ReplayThenAuthorize).await
    }

    async fn response_with_pipeline(
        req: Request,
        order: AuthorizationPipelineOrder,
    ) -> Result<Response, InternalError> {
        let ctx = Self::extract_root_context()?;
        crate::perf!("extract_context");
        let capability_req = RootCapabilityCommand::from(req);
        let capability = Self::map_request(capability_req);
        let capability_key = capability.metric_key();
        let capability_name = capability.capability_name();
        crate::perf!("map_request");

        let preflight = Self::preflight(&ctx, &capability, order)?;
        crate::perf!("preflight");
        let prepared = match preflight {
            RootPreflight::Fresh(prepared) => prepared,
            RootPreflight::Cached(response) => return Ok(response),
        };

        let response =
            match Self::execute_root_capability(&ctx, capability, prepared.authorized_cycles).await
            {
                Ok(response) => response,
                Err(err) => {
                    Self::abort_replay(prepared.pending);
                    RootCapabilityMetrics::record_execution(
                        capability_key,
                        RootCapabilityMetricOutcome::Error,
                    );
                    return Err(err);
                }
            };
        crate::perf!("execute_capability");
        if let Err(err) = Self::commit_replay(prepared.pending, &response) {
            log!(
                Topic::Rpc,
                Warn,
                "replay finalize failed after successful capability execution (capability={}, caller={}, subnet={}, now={}): {err}",
                capability_name,
                ctx.caller,
                ctx.subnet_id,
                ctx.now
            );
        }
        crate::perf!("commit_replay");
        RootCapabilityMetrics::record_execution(
            capability_key,
            RootCapabilityMetricOutcome::Success,
        );

        Ok(response)
    }

    fn preflight(
        ctx: &RootContext,
        capability: &RootCapability,
        order: AuthorizationPipelineOrder,
    ) -> Result<RootPreflight, InternalError> {
        match order {
            AuthorizationPipelineOrder::AuthorizeThenReplay => {
                let authorized_cycles = Self::authorize_with_hint(ctx, capability)?;
                match Self::check_replay(ctx, capability)? {
                    replay::ReplayPreflight::Fresh(pending) => {
                        Ok(RootPreflight::Fresh(PreparedExecution {
                            pending,
                            authorized_cycles,
                        }))
                    }
                    replay::ReplayPreflight::Cached(response) => {
                        Ok(RootPreflight::Cached(response))
                    }
                }
            }
            AuthorizationPipelineOrder::ReplayThenAuthorize => {
                match Self::check_replay(ctx, capability)? {
                    replay::ReplayPreflight::Fresh(pending) => {
                        let authorized_cycles = match Self::authorize_with_hint(ctx, capability) {
                            Ok(authorized_cycles) => authorized_cycles,
                            Err(err) => {
                                Self::abort_replay(pending);
                                return Err(err);
                            }
                        };
                        Ok(RootPreflight::Fresh(PreparedExecution {
                            pending,
                            authorized_cycles,
                        }))
                    }
                    replay::ReplayPreflight::Cached(response) => {
                        Ok(RootPreflight::Cached(response))
                    }
                }
            }
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
        capability: RootCapability,
        authorized_cycles: Option<nonroot_cycles::AuthorizedCyclesGrant>,
    ) -> Result<Response, InternalError> {
        execute::execute_root_capability(ctx, capability, authorized_cycles).await
    }

    fn check_replay(
        ctx: &RootContext,
        capability: &RootCapability,
    ) -> Result<replay::ReplayPreflight, InternalError> {
        replay::check_replay(ctx, capability)
    }

    fn commit_replay(pending: ReplayPending, response: &Response) -> Result<(), InternalError> {
        replay::commit_replay(pending, response)
    }

    fn abort_replay(pending: ReplayPending) {
        replay::abort_replay(pending);
    }

    #[cfg(test)]
    fn build_role_attestation(
        ctx: &RootContext,
        req: &crate::dto::auth::RoleAttestationRequest,
    ) -> Result<RoleAttestation, InternalError> {
        execute::build_role_attestation(ctx, req)
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

    fn map_request(req: RootCapabilityCommand) -> RootCapability {
        capability::map_request(req)
    }
}

#[cfg(test)]
fn hash_domain_separated(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    replay::hash_domain_separated(domain, payload)
}
