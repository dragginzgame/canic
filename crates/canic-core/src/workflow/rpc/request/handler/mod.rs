#[cfg(test)]
use crate::dto::auth::RoleAttestation;
#[cfg(test)]
use crate::storage::stable::replay::ReplaySlotKey;
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::rpc::{Request, Response, RootCapabilityCommand},
    ops::{
        ic::IcOps,
        replay::guard::ReplayPending,
        runtime::env::EnvOps,
        runtime::metrics::root_capability::{
            RootCapabilityMetricEventType, RootCapabilityMetricOutcome, RootCapabilityMetrics,
        },
    },
};

mod authorize;
mod capability;
mod delegation;
mod execute;
mod replay;
use capability::RootCapability;

const REPLAY_PURGE_SCAN_LIMIT: usize = 256;
const MAX_ROOT_REPLAY_ENTRIES: usize = 10_000;
const MAX_ROOT_TTL_SECONDS: u64 = 300;
const DEFAULT_MAX_ROLE_ATTESTATION_TTL_SECONDS: u64 = 900;
const REPLAY_PAYLOAD_HASH_DOMAIN: &[u8] = b"root-replay-payload-hash:v1";

///
/// RootResponseWorkflow
///

pub struct RootResponseWorkflow;

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
enum AuthorizationPipelineOrder {
    AuthorizeThenReplay,
    ReplayThenAuthorize,
}

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
        let capability_req = RootCapabilityCommand::from(req);
        let capability = Self::map_request(capability_req);
        let capability_key = capability.metric_key();

        let pending = Self::preflight(&ctx, &capability, order)?;

        let response = match Self::execute_root_capability(&ctx, capability).await {
            Ok(response) => response,
            Err(err) => {
                RootCapabilityMetrics::record(
                    capability_key,
                    RootCapabilityMetricEventType::Execution,
                    RootCapabilityMetricOutcome::Error,
                );
                return Err(err);
            }
        };
        if let Err(err) = Self::commit_replay(pending, &response) {
            RootCapabilityMetrics::record(
                capability_key,
                RootCapabilityMetricEventType::Execution,
                RootCapabilityMetricOutcome::Error,
            );
            return Err(err);
        }
        RootCapabilityMetrics::record(
            capability_key,
            RootCapabilityMetricEventType::Execution,
            RootCapabilityMetricOutcome::Success,
        );

        Ok(response)
    }

    fn preflight(
        ctx: &RootContext,
        capability: &RootCapability,
        order: AuthorizationPipelineOrder,
    ) -> Result<ReplayPending, InternalError> {
        match order {
            AuthorizationPipelineOrder::AuthorizeThenReplay => {
                Self::authorize(ctx, capability)?;
                Self::check_replay(ctx, capability)
            }
            AuthorizationPipelineOrder::ReplayThenAuthorize => {
                let replay = Self::check_replay(ctx, capability)?;
                Self::authorize(ctx, capability)?;
                Ok(replay)
            }
        }
    }

    fn extract_root_context() -> Result<RootContext, InternalError> {
        EnvOps::require_root()?;

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

    fn authorize(ctx: &RootContext, capability: &RootCapability) -> Result<(), InternalError> {
        authorize::authorize(ctx, capability)
    }

    async fn execute_root_capability(
        ctx: &RootContext,
        capability: RootCapability,
    ) -> Result<Response, InternalError> {
        execute::execute_root_capability(ctx, capability).await
    }

    fn check_replay(
        ctx: &RootContext,
        capability: &RootCapability,
    ) -> Result<ReplayPending, InternalError> {
        replay::check_replay(ctx, capability)
    }

    fn commit_replay(pending: ReplayPending, response: &Response) -> Result<(), InternalError> {
        replay::commit_replay(pending, response)
    }

    #[cfg(test)]
    fn build_role_attestation(
        ctx: &RootContext,
        req: &crate::dto::auth::RoleAttestationRequest,
    ) -> Result<RoleAttestation, InternalError> {
        execute::build_role_attestation(ctx, req)
    }
}

#[cfg(test)]
fn replay_slot_key(
    caller: Principal,
    target_canister: Principal,
    request_id: [u8; 32],
) -> ReplaySlotKey {
    replay::replay_slot_key(caller, target_canister, request_id)
}

#[cfg(test)]
fn replay_slot_key_legacy(
    caller: Principal,
    subnet_id: Principal,
    request_id: [u8; 32],
) -> ReplaySlotKey {
    replay::replay_slot_key_legacy(caller, subnet_id, request_id)
}

#[cfg(test)]
fn hash_domain_separated(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    replay::hash_domain_separated(domain, payload)
}

#[cfg(test)]
mod tests;
