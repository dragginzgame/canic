//! Module: workflow::ic::icp_refill
//!
//! Responsibility: orchestrate ICP-to-cycles refill execution.
//! Does not own: endpoint auth, stable record mutation, or pure refill policy.
//! Boundary: calls policy, IC ops, storage ops, and replay/cost-guard helpers.

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
    domain::policy::icp_refill::{
        IcpRefillPolicyInput, IcpRefillPolicyRules, IcpRefillPolicyViolation,
        evaluate_manual_refill,
    },
    dto::icp_refill::IcpRefillRequest,
    ids::BuildNetwork,
    infra::ic::icp_refill::IcpRefillCanisterOverrides,
    ops::{
        config::ConfigOps,
        ic::{IcOps, icp_refill::IcpRefillOps},
        runtime::cycles_funding::CyclesFundingLedgerOps,
        storage::{icp_refill::IcpRefillStoreOps, state::app::AppStateOps},
    },
    workflow::ic::network::NetworkWorkflow,
};
use thiserror::Error as ThisError;

use self::execution::direct_child_refill_role;

const TX_WINDOW_NANOS: u64 = 24 * 60 * 60 * 1_000_000_000;
const MAX_NOTIFY_ATTEMPTS: u32 = 5;
const ICP_LEDGER_DECIMALS: u8 = 8;
const ICP_REFILL_REPLAY_COMMAND_KIND: &str = "icp.refill.v1";

///
/// IcpRefillWorkflowError
///
/// Typed workflow-layer failure for ICP refill orchestration.
/// Owned by ICP refill workflow and converted into internal workflow errors.
///

#[derive(Debug, ThisError)]
pub enum IcpRefillWorkflowError {
    #[error("ICP refill request is marked dry_run; call dry_run_manual_refill instead")]
    DryRunRequest,

    #[error("ICP refill Nat field {field} does not fit in u64: {value}")]
    NatU64Overflow { field: &'static str, value: Nat },

    #[error("ICP refill policy denied request: {0:?}")]
    PolicyDenied(IcpRefillPolicyViolation),

    #[error("ICP refill source canister {source_canister} must be this canister {self_pid}")]
    SourceCanisterMismatch {
        source_canister: Principal,
        self_pid: Principal,
    },

    #[error("ICP refill expected ICP ledger decimals=8, found {0}")]
    UnexpectedLedgerDecimals(u8),

    #[error("ICP refill only supports canister mode in this workflow")]
    UnsupportedMode,
}

impl From<IcpRefillWorkflowError> for InternalError {
    fn from(err: IcpRefillWorkflowError) -> Self {
        Self::workflow(InternalErrorOrigin::Workflow, err.to_string())
    }
}

///
/// IcpRefillWorkflow
///
/// Workflow entrypoint for manual and automatic ICP refill orchestration.
/// Owned by workflow and called after endpoints authenticate input.
///

pub struct IcpRefillWorkflow;

///
/// IcpRefillExecutionContext
///
/// Prepared IC canister IDs and fee/rate context for one refill execution.
/// Owned by workflow and passed into execution helpers.
///

struct IcpRefillExecutionContext {
    ledger_canister_id: Principal,
    cmc_canister_id: Principal,
    fee_e8s: u64,
    xdr_permyriad_per_icp: Option<u64>,
    created_at_time_ns: u64,
}

///
/// ManualRefillPreflight
///
/// Cached policy preflight input for manual refill requests.
/// Owned by workflow and evaluated before mutation or IC calls proceed.
///

struct ManualRefillPreflight {
    policy: Option<IcpRefillPolicyRules>,
    input: IcpRefillPolicyInput,
    rate_gate_configured: bool,
}

impl ManualRefillPreflight {
    fn new(policy: Option<&IcpRefillPolicy>, request: &IcpRefillRequest) -> Self {
        let input = policy_input(
            0,
            request,
            None,
            in_flight_for_request(request),
            AppStateOps::cycles_funding_enabled(),
            funding_cooldown_retry_after_secs(request, IcOps::now_secs()),
        );
        let rate_gate_configured = policy_requires_rate(policy);
        let policy = policy.map(icp_refill_policy_rules);

        Self {
            policy,
            input,
            rate_gate_configured,
        }
    }

    fn evaluate(&self, observed_xdr_permyriad_per_icp: Option<u64>) -> Result<(), InternalError> {
        evaluate_manual_refill(
            self.policy.as_ref(),
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
/// Controls whether workflow must query the CMC conversion rate.
/// Owned by ICP refill workflow policy preparation.
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
    let preflight = ManualRefillPreflight::new(policy.as_ref(), request);
    if !preflight.rate_gate_configured {
        preflight.evaluate(None)?;
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

    if preflight.rate_gate_configured {
        preflight.evaluate(xdr_permyriad_per_icp)?;
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

fn icp_refill_policy_rules(policy: &IcpRefillPolicy) -> IcpRefillPolicyRules {
    IcpRefillPolicyRules {
        enabled: policy.enabled,
        min_hub_cycles_before_refill: policy.min_hub_cycles_before_refill.clone(),
        max_refill_e8s_per_call: policy.max_refill_e8s_per_call,
        min_xdr_permyriad_per_icp: policy.min_xdr_permyriad_per_icp,
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

    ConfigOps::cycles_funding_policy_for_child_role(&role)
        .ok()?
        .cooldown_retry_after_secs(
            CyclesFundingLedgerOps::snapshot(request.target_canister),
            now_secs,
        )
}

fn policy_denied(violation: IcpRefillPolicyViolation) -> InternalError {
    IcpRefillWorkflowError::PolicyDenied(violation).into()
}

fn in_flight_for_request(request: &IcpRefillRequest) -> bool {
    IcpRefillStoreOps::has_in_flight_for_key(
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
