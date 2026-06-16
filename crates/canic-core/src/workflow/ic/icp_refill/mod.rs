mod cost_guard;
mod execution;
mod hub;
mod manual;
mod replay;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::{
        candid::Nat,
        types::{Cycles, Principal},
    },
    config::schema::{IcpRefillPolicy, TopupPolicy},
    domain::policy::{
        cycles_funding,
        icp_refill::{IcpRefillPolicyInput, IcpRefillPolicyViolation, evaluate_manual_refill},
    },
    dto::icp_refill::IcpRefillRequest,
    ids::BuildNetwork,
    infra::ic::icp_refill::IcpRefillCanisterOverrides,
    ops::{
        config::ConfigOps,
        ic::{IcOps, icp_refill::IcpRefillOps},
        runtime::cycles_funding::CyclesFundingLedgerOps,
        storage::{icp_refill::IcpRefillRecordOps, state::app::AppStateOps},
    },
    workflow::ic::network::NetworkWorkflow,
};
use thiserror::Error as ThisError;

use self::execution::direct_child_refill_role;

const TX_WINDOW_NANOS: u64 = 24 * 60 * 60 * 1_000_000_000;
const MAX_NOTIFY_ATTEMPTS: u32 = 5;
const ICP_LEDGER_DECIMALS: u8 = 8;
const ICP_REFILL_REPLAY_COMMAND_KIND: &str = "icp.refill.v1";
const ICP_REFILL_REPLAY_RESPONSE_SCHEMA_VERSION: u32 = 1;

///
/// IcpRefillWorkflowError
///

#[derive(Debug, ThisError)]
pub enum IcpRefillWorkflowError {
    #[error("ICP refill only supports canister mode in this workflow")]
    UnsupportedMode,

    #[error("ICP refill source canister {source_canister} must be this canister {self_pid}")]
    SourceCanisterMismatch {
        source_canister: Principal,
        self_pid: Principal,
    },

    #[error("ICP refill request is marked dry_run; call dry_run_manual_refill instead")]
    DryRunRequest,

    #[error("ICP refill policy denied request: {0:?}")]
    PolicyDenied(IcpRefillPolicyViolation),

    #[error("ICP refill Nat field {field} does not fit in u64: {value}")]
    NatU64Overflow { field: &'static str, value: Nat },

    #[error("ICP refill expected ICP ledger decimals=8, found {0}")]
    UnexpectedLedgerDecimals(u8),
}

impl From<IcpRefillWorkflowError> for InternalError {
    fn from(err: IcpRefillWorkflowError) -> Self {
        Self::workflow(InternalErrorOrigin::Workflow, err.to_string())
    }
}

///
/// IcpRefillWorkflow
///

pub struct IcpRefillWorkflow;

///
/// IcpRefillExecutionContext
///

struct IcpRefillExecutionContext {
    ledger_canister_id: Principal,
    cmc_canister_id: Principal,
    fee_e8s: u64,
    xdr_permyriad_per_icp: Option<u64>,
    created_at_time_ns: u64,
}

///
/// ManualRefillPolicyPreflight
///

struct ManualRefillPolicyPreflight<'a> {
    policy: Option<&'a IcpRefillPolicy>,
    input: IcpRefillPolicyInput,
    rate_gate_configured: bool,
}

impl<'a> ManualRefillPolicyPreflight<'a> {
    fn new(policy: Option<&'a IcpRefillPolicy>, request: &IcpRefillRequest) -> Self {
        let input = policy_input(
            0,
            request,
            None,
            in_flight_for_request(request),
            AppStateOps::cycles_funding_enabled(),
            funding_cooldown_retry_after_secs(request, IcOps::now_secs()),
        );

        Self {
            policy,
            input,
            rate_gate_configured: policy_requires_rate(policy),
        }
    }

    fn evaluate(&self, observed_xdr_permyriad_per_icp: Option<u64>) -> Result<(), InternalError> {
        evaluate_manual_refill(
            self.policy,
            IcpRefillPolicyInput {
                observed_xdr_permyriad_per_icp,
                ..self.input
            },
        )
        .map(|_decision| ())
        .map_err(policy_denied)
    }
}

///
/// RateQueryMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RateQueryMode {
    Always,
    WhenGateConfigured,
}

async fn prepare_context(
    request: &IcpRefillRequest,
    rate_query_mode: RateQueryMode,
) -> Result<IcpRefillExecutionContext, InternalError> {
    let policy = current_icp_refill_policy()?;
    let policy_preflight = ManualRefillPolicyPreflight::new(policy.as_ref(), request);
    if !policy_preflight.rate_gate_configured {
        policy_preflight.evaluate(None)?;
    }

    let canisters = IcpRefillOps::resolve_canisters(
        build_network(),
        refill_canister_overrides(policy.as_ref()),
    )?;
    let fee = IcpRefillOps::icrc1_fee(canisters.ledger_canister_id).await?;
    let fee_e8s = checked_nat_u64("icrc1_fee", fee)?;
    validate_ledger_decimals(IcpRefillOps::icrc1_decimals(canisters.ledger_canister_id).await?)?;
    let xdr_permyriad_per_icp =
        configured_rate(policy.as_ref(), canisters.cmc_canister_id, rate_query_mode).await?;

    if policy_preflight.rate_gate_configured {
        policy_preflight.evaluate(xdr_permyriad_per_icp)?;
    }

    Ok(IcpRefillExecutionContext {
        ledger_canister_id: canisters.ledger_canister_id,
        cmc_canister_id: canisters.cmc_canister_id,
        fee_e8s,
        xdr_permyriad_per_icp,
        created_at_time_ns: IcOps::now_nanos(),
    })
}

async fn configured_rate(
    policy: Option<&IcpRefillPolicy>,
    cmc_canister_id: Principal,
    mode: RateQueryMode,
) -> Result<Option<u64>, InternalError> {
    if !rate_required(policy, mode) {
        return Ok(None);
    }

    let response = IcpRefillOps::get_icp_xdr_conversion_rate(cmc_canister_id).await?;
    Ok(Some(response.data.xdr_permyriad_per_icp))
}

const fn policy_requires_rate(policy: Option<&IcpRefillPolicy>) -> bool {
    matches!(
        policy,
        Some(IcpRefillPolicy {
            min_xdr_permyriad_per_icp: Some(_),
            ..
        })
    )
}

const fn rate_required(policy: Option<&IcpRefillPolicy>, mode: RateQueryMode) -> bool {
    matches!(mode, RateQueryMode::Always) || policy_requires_rate(policy)
}

fn refill_canister_overrides(policy: Option<&IcpRefillPolicy>) -> IcpRefillCanisterOverrides {
    let Some(policy) = policy else {
        return IcpRefillCanisterOverrides::default();
    };

    IcpRefillCanisterOverrides {
        ledger_canister_id: policy.ledger_canister_id,
        cmc_canister_id: policy.cmc_canister_id,
        allow_ic_overrides: policy.allow_ic_system_canister_overrides,
    }
}

fn validate_ledger_decimals(decimals: u8) -> Result<(), InternalError> {
    if decimals == ICP_LEDGER_DECIMALS {
        Ok(())
    } else {
        Err(IcpRefillWorkflowError::UnexpectedLedgerDecimals(decimals).into())
    }
}

fn estimate_cycles(amount_e8s: u64, xdr_permyriad_per_icp: u64) -> Cycles {
    Cycles::new(u128::from(amount_e8s).saturating_mul(u128::from(xdr_permyriad_per_icp)))
}

fn current_icp_refill_policy() -> Result<Option<IcpRefillPolicy>, InternalError> {
    Ok(ConfigOps::current_canister()?
        .topup
        .and_then(|topup| topup.icp_refill))
}

fn current_topup_policy() -> Result<Option<TopupPolicy>, InternalError> {
    Ok(ConfigOps::current_canister()?.topup)
}

const fn policy_input(
    hub_cycles: u128,
    request: &IcpRefillRequest,
    observed_xdr_permyriad_per_icp: Option<u64>,
    in_flight_for_key: bool,
    cycles_funding_enabled: bool,
    funding_cooldown_retry_after_secs: Option<u64>,
) -> IcpRefillPolicyInput {
    IcpRefillPolicyInput {
        hub_cycles,
        requested_amount_e8s: request.amount_e8s,
        observed_xdr_permyriad_per_icp,
        in_flight_for_key,
        cycles_funding_enabled,
        funding_cooldown_retry_after_secs,
    }
}

fn funding_cooldown_retry_after_secs(request: &IcpRefillRequest, now_secs: u64) -> Option<u64> {
    let role = direct_child_refill_role(request.target_canister, request.source_canister)?;

    cycles_funding::policy_for_child_role(&role).cooldown_retry_after_secs(
        CyclesFundingLedgerOps::snapshot(request.target_canister),
        now_secs,
    )
}

fn policy_denied(violation: IcpRefillPolicyViolation) -> InternalError {
    IcpRefillWorkflowError::PolicyDenied(violation).into()
}

fn in_flight_for_request(request: &IcpRefillRequest) -> bool {
    IcpRefillRecordOps::has_in_flight_for_key(
        request.source_canister,
        request.source_subaccount,
        request.target_canister,
        request.operation_id,
    )
}

fn build_network() -> BuildNetwork {
    NetworkWorkflow::build_network().unwrap_or(BuildNetwork::Local)
}

fn checked_nat_u64(field: &'static str, value: Nat) -> Result<u64, InternalError> {
    u64::try_from(value.0.clone())
        .map_err(|_| IcpRefillWorkflowError::NatU64Overflow { field, value }.into())
}

#[cfg(test)]
mod tests;
