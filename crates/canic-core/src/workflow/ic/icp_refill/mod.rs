//! Module: workflow::ic::icp_refill
//!
//! Responsibility: orchestrate ICP-to-cycles refill execution.
//! Does not own: endpoint auth, stable record mutation, or pure refill policy.
//! Boundary: calls policy, IC ops, storage ops, and replay/cost-guard helpers.

mod automatic;
mod cost_guard;
mod execution;
mod manual;
mod replay;

use crate::{
    InternalError,
    cdk::{
        candid::Nat,
        types::{Cycles, Principal},
    },
    domain::icp_refill::IcpRefillTrigger,
    domain::policy::pure::icp_refill::{
        AutomaticIcpRefillAmountError, AutomaticIcpRefillRules, AutomaticIcpRefillUsage,
        IcpRefillPolicyInput, IcpRefillPolicyRules, IcpRefillPolicyViolation,
        evaluate_automatic_refill, evaluate_manual_refill,
    },
    dto::icp_refill::IcpRefillRequest,
    ids::{BuildNetwork, FleetSubnetRootIcpRefillPolicy},
    infra::ic::icp_refill::{IcpRefillCanisterOverrides, Icrc1Account},
    ops::{
        ic::{IcOps, build_network::BuildNetworkOps, icp_refill::IcpRefillOps},
        storage::{
            fleet_activation::FleetActivationOps,
            icp_refill::{IcpRefillPolicyUsage, IcpRefillStoreOps},
            state::fleet::FleetStateOps,
        },
    },
};
use thiserror::Error as ThisError;

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

    #[error("ICP refill expected ICP ledger decimals=8, found {0}")]
    UnexpectedLedgerDecimals(u8),

    #[error("automatic ICP refill amount derivation failed: {0:?}")]
    AutomaticAmount(AutomaticIcpRefillAmountError),

    #[error(
        "automatic ICP refill requires balance at or below emergency threshold: current={current_cycles}, threshold={emergency_threshold}"
    )]
    AutomaticThresholdNotMet {
        current_cycles: u128,
        emergency_threshold: u128,
    },
}

impl From<IcpRefillWorkflowError> for InternalError {
    fn from(err: IcpRefillWorkflowError) -> Self {
        match err {
            IcpRefillWorkflowError::AutomaticThresholdNotMet { .. } => {
                Self::public(crate::diagnostics::codes::PLATFORM_INVALID_STATE)
            }
            IcpRefillWorkflowError::AutomaticAmount(
                AutomaticIcpRefillAmountError::TargetSatisfied,
            ) => Self::public(crate::diagnostics::codes::PLATFORM_INVALID_STATE),
            IcpRefillWorkflowError::AutomaticAmount(
                AutomaticIcpRefillAmountError::AmountOverflow { .. }
                | AutomaticIcpRefillAmountError::RateZero,
            ) => Self::public(crate::diagnostics::codes::CAPACITY_INVALID),
            IcpRefillWorkflowError::DryRunRequest => {
                Self::public(crate::diagnostics::codes::PLATFORM_INACTIVE)
            }
            IcpRefillWorkflowError::NatU64Overflow { .. }
            | IcpRefillWorkflowError::UnexpectedLedgerDecimals(_) => {
                Self::public(crate::diagnostics::codes::EVIDENCE_INVALID)
            }
            IcpRefillWorkflowError::PolicyDenied(violation) => match violation {
                IcpRefillPolicyViolation::NotConfigured
                | IcpRefillPolicyViolation::ConcurrentRefill => {
                    Self::public(crate::diagnostics::codes::PLATFORM_INVALID_STATE)
                }
                IcpRefillPolicyViolation::CyclesFundingDisabled => {
                    Self::public(crate::diagnostics::codes::CAPACITY_INACTIVE)
                }
                IcpRefillPolicyViolation::AmountZero
                | IcpRefillPolicyViolation::AmountAndFeeOverflow => {
                    Self::public(crate::diagnostics::codes::CAPACITY_INVALID)
                }
                IcpRefillPolicyViolation::MaxRefillPerCall { .. }
                | IcpRefillPolicyViolation::WindowBudgetExhausted { .. }
                | IcpRefillPolicyViolation::AutomaticRefillCountExhausted { .. }
                | IcpRefillPolicyViolation::AutomaticRefillSpendExhausted { .. } => {
                    Self::public(crate::diagnostics::codes::CAPACITY_LIMIT)
                }
                IcpRefillPolicyViolation::BalanceFloorUnavailable { .. }
                | IcpRefillPolicyViolation::RateUnavailable { .. } => {
                    Self::public(crate::diagnostics::codes::CAPACITY_UNAVAILABLE)
                }
                IcpRefillPolicyViolation::RateGateDenied { .. } => {
                    Self::public(crate::diagnostics::codes::CAPACITY_INVALID_STATE)
                }
            },
        }
    }
}

///
/// IcpRefillWorkflow
///
/// Workflow entrypoint for explicit ICP refill orchestration.
/// Owned by workflow and called after endpoints authenticate input.
///

pub struct IcpRefillWorkflow;

///
/// IcpRefillExecutionContext
///
/// Prepared IC canister IDs and fee/rate context for one refill execution.
/// Owned by workflow and passed into execution helpers.
///

pub(super) struct IcpRefillExecutionContext {
    ledger_canister_id: Principal,
    cmc_canister_id: Principal,
    fee_e8s: u64,
    xdr_permyriad_per_icp: Option<u64>,
    budget_window_start_secs: u64,
    policy_hash: [u8; 32],
    created_at_time_ns: u64,
}

///
/// RefillPreflight
///
/// Point-in-time policy preflight input for manual refill requests.
/// Owned by workflow and rebuilt after asynchronous calls before mutation proceeds.
///

struct RefillPreflight {
    policy: Option<IcpRefillPolicyRules>,
    automatic: Option<AutomaticIcpRefillRules>,
    automatic_usage: AutomaticIcpRefillUsage,
    input: IcpRefillPolicyInput,
    rate_gate_configured: bool,
}

impl RefillPreflight {
    fn new(
        policy: Option<&FleetSubnetRootIcpRefillPolicy>,
        request: &IcpRefillRequest,
        root_canister: Principal,
    ) -> Result<Self, InternalError> {
        let input = policy_input(
            request,
            None,
            None,
            None,
            policy.map_or(0, |policy| {
                let window_start_secs = fixed_window_start(IcOps::now_secs(), policy.window_secs);
                IcpRefillStoreOps::policy_usage(window_start_secs).window_reserved_e8s
            }),
            active_for_request(request, root_canister)?,
            FleetStateOps::cycles_funding_enabled(),
        );
        let rate_gate_configured = policy_requires_rate(policy);
        let usage = policy.map_or_else(IcpRefillPolicyUsage::default, |policy| {
            IcpRefillStoreOps::policy_usage(fixed_window_start(
                IcOps::now_secs(),
                policy.window_secs,
            ))
        });
        let automatic = policy
            .and_then(|policy| policy.automatic.as_ref())
            .map(automatic_icp_refill_rules);
        let policy = policy.map(icp_refill_policy_rules);
        Ok(Self {
            policy,
            automatic,
            automatic_usage: AutomaticIcpRefillUsage {
                completed_refills: usage.automatic_completed_refills,
                completed_refill_e8s: usage.automatic_completed_refill_e8s,
            },
            input,
            rate_gate_configured,
        })
    }

    fn evaluate(
        &self,
        trigger: IcpRefillTrigger,
        observed_xdr_permyriad_per_icp: Option<u64>,
    ) -> Result<(), InternalError> {
        let input = IcpRefillPolicyInput {
            observed_xdr_permyriad_per_icp,
            ..self.input
        };
        match trigger {
            IcpRefillTrigger::Manual => evaluate_manual_refill(self.policy.as_ref(), input),
            IcpRefillTrigger::Automatic { .. } => evaluate_automatic_refill(
                self.policy.as_ref(),
                self.automatic.as_ref(),
                self.automatic_usage,
                input,
            ),
        }
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
    root_canister: Principal,
    rate_query_mode: RateQueryMode,
    trigger: IcpRefillTrigger,
) -> Result<IcpRefillExecutionContext, InternalError> {
    let policy = current_icp_refill_policy()?;
    let policy_hash = current_root_funding_policy_hash()?;
    let preflight = RefillPreflight::new(policy.as_ref(), request, root_canister)?;
    if !preflight.rate_gate_configured {
        preflight.evaluate(trigger, None)?;
    }

    let canisters = IcpRefillOps::resolve_canisters(
        require_build_network(BuildNetworkOps::build_network())?,
        refill_canister_overrides(policy.as_ref()),
    )?;
    let fee = IcpRefillOps::icrc1_fee(canisters.ledger_canister_id).await?;
    let fee_e8s = checked_nat_u64("icrc1_fee", fee)?;
    validate_ledger_decimals(IcpRefillOps::icrc1_decimals(canisters.ledger_canister_id).await?)?;
    let source_balance_e8s = checked_nat_u64(
        "icrc1_balance_of",
        IcpRefillOps::icrc1_balance_of(
            canisters.ledger_canister_id,
            Icrc1Account {
                owner: root_canister,
                subaccount: request.source_subaccount,
            },
        )
        .await?,
    )?;
    let xdr_permyriad_per_icp =
        configured_rate(policy.as_ref(), canisters.cmc_canister_id, rate_query_mode).await?;

    let current_policy = current_icp_refill_policy()?;
    if current_policy != policy || current_root_funding_policy_hash()? != policy_hash {
        return Err(InternalError::public(
            crate::diagnostics::codes::STATE_CONFLICT,
        ));
    }
    let window_start_secs = policy.as_ref().map_or(0, |policy| {
        fixed_window_start(IcOps::now_secs(), policy.window_secs)
    });
    let usage = IcpRefillStoreOps::policy_usage(window_start_secs);
    let mut final_preflight = RefillPreflight::new(policy.as_ref(), request, root_canister)?;
    final_preflight.input.observed_fee_e8s = Some(fee_e8s);
    final_preflight.input.observed_source_balance_e8s = Some(source_balance_e8s);
    final_preflight.input.window_reserved_e8s = usage.window_reserved_e8s;
    final_preflight.evaluate(trigger, xdr_permyriad_per_icp)?;

    Ok(IcpRefillExecutionContext {
        ledger_canister_id: canisters.ledger_canister_id,
        cmc_canister_id: canisters.cmc_canister_id,
        fee_e8s,
        xdr_permyriad_per_icp,
        budget_window_start_secs: window_start_secs,
        policy_hash,
        created_at_time_ns: IcOps::now_nanos(),
    })
}

async fn configured_rate(
    policy: Option<&FleetSubnetRootIcpRefillPolicy>,
    cmc_canister_id: Principal,
    mode: RateQueryMode,
) -> Result<Option<u64>, InternalError> {
    if !rate_required(policy, mode) {
        return Ok(None);
    }

    let response = IcpRefillOps::get_icp_xdr_conversion_rate(cmc_canister_id).await?;
    Ok(Some(response.data.xdr_permyriad_per_icp))
}

const fn policy_requires_rate(policy: Option<&FleetSubnetRootIcpRefillPolicy>) -> bool {
    matches!(
        policy,
        Some(FleetSubnetRootIcpRefillPolicy {
            min_xdr_permyriad_per_icp: Some(_),
            ..
        })
    )
}

const fn rate_required(
    policy: Option<&FleetSubnetRootIcpRefillPolicy>,
    mode: RateQueryMode,
) -> bool {
    matches!(mode, RateQueryMode::Always) || policy_requires_rate(policy)
}

fn refill_canister_overrides(
    policy: Option<&FleetSubnetRootIcpRefillPolicy>,
) -> IcpRefillCanisterOverrides {
    let Some(policy) = policy else {
        return IcpRefillCanisterOverrides::default();
    };

    IcpRefillCanisterOverrides {
        ledger_canister_id: policy.ledger_canister_id,
        cmc_canister_id: policy.cmc_canister_id,
        allow_ic_overrides: policy.allow_ic_system_canister_overrides,
    }
}

const fn icp_refill_policy_rules(policy: &FleetSubnetRootIcpRefillPolicy) -> IcpRefillPolicyRules {
    IcpRefillPolicyRules {
        max_refill_e8s_per_call: policy.max_refill_e8s_per_call,
        maximum_refill_e8s: policy.maximum_refill_e8s,
        minimum_icp_balance_e8s: policy.minimum_icp_balance_e8s,
        min_xdr_permyriad_per_icp: policy.min_xdr_permyriad_per_icp,
    }
}

const fn automatic_icp_refill_rules(
    policy: &crate::ids::FleetSubnetRootAutomaticIcpRefillPolicy,
) -> AutomaticIcpRefillRules {
    AutomaticIcpRefillRules {
        maximum_automatic_refills: policy.maximum_automatic_refills,
        maximum_automatic_refill_e8s: policy.maximum_automatic_refill_e8s,
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

fn current_icp_refill_policy() -> Result<Option<FleetSubnetRootIcpRefillPolicy>, InternalError> {
    FleetActivationOps::root_authority()
        .map(|authority| authority.binding.funding.icp_refill)
        .map_err(crate::ops::storage::StorageOpsError::from)
        .map_err(Into::into)
}

fn current_root_funding_policy_hash() -> Result<[u8; 32], InternalError> {
    FleetActivationOps::root_authority()
        .map(|authority| {
            crate::ops::fleet_funding_policy::fleet_subnet_root_funding_policy_hash(
                &authority.binding.funding,
            )
        })
        .map_err(crate::ops::storage::StorageOpsError::from)
        .map_err(Into::into)
}

fn require_icp_refill_configured() -> Result<(), InternalError> {
    let policy = current_icp_refill_policy()?;
    validate_icp_refill_configured(policy.as_ref())
}

fn validate_icp_refill_configured(
    policy: Option<&FleetSubnetRootIcpRefillPolicy>,
) -> Result<(), InternalError> {
    if policy.is_some() {
        Ok(())
    } else {
        Err(policy_denied(IcpRefillPolicyViolation::NotConfigured))
    }
}

const fn policy_input(
    request: &IcpRefillRequest,
    observed_xdr_permyriad_per_icp: Option<u64>,
    observed_fee_e8s: Option<u64>,
    observed_source_balance_e8s: Option<u64>,
    window_reserved_e8s: u64,
    active_for_key: bool,
    cycles_funding_enabled: bool,
) -> IcpRefillPolicyInput {
    IcpRefillPolicyInput {
        requested_amount_e8s: request.amount_e8s,
        observed_xdr_permyriad_per_icp,
        observed_fee_e8s,
        observed_source_balance_e8s,
        window_reserved_e8s,
        active_for_key,
        cycles_funding_enabled,
    }
}

fn policy_denied(violation: IcpRefillPolicyViolation) -> InternalError {
    IcpRefillWorkflowError::PolicyDenied(violation).into()
}

fn active_for_request(
    request: &IcpRefillRequest,
    root_canister: Principal,
) -> Result<bool, InternalError> {
    IcpRefillStoreOps::has_active_for_root(root_canister, root_canister, request.operation_id)
}

const fn fixed_window_start(now_secs: u64, window_secs: u64) -> u64 {
    if window_secs == 0 {
        return 0;
    }
    now_secs / window_secs * window_secs
}

fn require_build_network(
    build_network: Option<BuildNetwork>,
) -> Result<BuildNetwork, InternalError> {
    build_network
        .ok_or_else(|| InternalError::public(crate::diagnostics::codes::PLATFORM_UNAVAILABLE))
}

fn checked_nat_u64(field: &'static str, value: Nat) -> Result<u64, InternalError> {
    u64::try_from(value.0.clone())
        .map_err(|_| IcpRefillWorkflowError::NatU64Overflow { field, value }.into())
}

#[cfg(test)]
mod tests;
