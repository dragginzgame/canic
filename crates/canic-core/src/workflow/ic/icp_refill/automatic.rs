//! Module: workflow::ic::icp_refill::automatic
//!
//! Responsibility: prepare, start, and resume lower-threshold automatic Root ICP refills.
//! Does not own: Coordinator no-grant classification, timer scheduling, or ledger mechanics.
//! Boundary: the sole Root cycles timer calls this only after a terminal eligible no-grant.

use crate::{
    InternalError,
    cdk::types::Principal,
    domain::{
        icp_refill::IcpRefillTrigger,
        policy::pure::icp_refill::{IcpRefillPolicyViolation, automatic_refill_amount_e8s},
    },
    dto::icp_refill::{IcpRefillRequest, IcpRefillResponse},
    infra::ic::icp_refill::Icrc1Account,
    ops::{
        ic::{IcOps, build_network::BuildNetworkOps, icp_refill::IcpRefillOps},
        storage::{icp_refill::IcpRefillStoreOps, state::fleet::FleetStateOps},
    },
    workflow::ic::icp_refill::{
        ICP_LEDGER_DECIMALS, IcpRefillExecutionContext, IcpRefillWorkflow, IcpRefillWorkflowError,
        RateQueryMode, RefillPreflight, checked_nat_u64, current_icp_refill_policy,
        current_root_funding_policy_hash,
        execution::execute_fresh_automatic_refill,
        fixed_window_start, refill_canister_overrides,
        replay::{
            IcpRefillReplayReservation, icp_refill_replay_reserve_input,
            log_icp_refill_committed_replay, log_icp_refill_fresh_reservation,
            reserve_icp_refill_replay,
        },
        require_build_network, validate_icp_refill_configured, validate_ledger_decimals,
    },
};
use sha2::{Digest, Sha256};

const AUTOMATIC_OPERATION_ID_DOMAIN: &[u8] = b"canic.icp-refill.automatic.v1";

struct PreparedAutomaticRefill {
    request: IcpRefillRequest,
    sequence: u64,
    context: IcpRefillExecutionContext,
}

impl IcpRefillWorkflow {
    /// Return the protected lower threshold, or `None` when automatic refill is absent.
    pub(crate) fn automatic_refill_threshold() -> Result<Option<u128>, InternalError> {
        Ok(current_icp_refill_policy()?
            .and_then(|policy| policy.automatic)
            .map(|automatic| automatic.emergency_threshold.to_u128()))
    }

    /// Return the durable owner of the one active Root refill, when present.
    pub(crate) fn active_refill_trigger() -> Result<Option<IcpRefillTrigger>, InternalError> {
        Ok(IcpRefillStoreOps::active_operation()?.map(|operation| operation.trigger))
    }

    /// Resume one durable timer-owned operation before any new Coordinator request.
    pub(crate) async fn resume_automatic_refill() -> Result<IcpRefillResponse, InternalError> {
        let operation = IcpRefillStoreOps::active_operation()?
            .ok_or_else(|| InternalError::public(crate::diagnostics::codes::STATE_UNAVAILABLE))?;
        let IcpRefillTrigger::Automatic { sequence } = operation.trigger else {
            return Err(super::policy_denied(
                IcpRefillPolicyViolation::ConcurrentRefill,
            ));
        };
        let root_canister = IcOps::canister_self();
        let request = IcpRefillRequest {
            operation_id: operation.operation_id,
            source_subaccount: operation.source_subaccount,
            amount_e8s: operation.amount_e8s,
            dry_run: false,
        };
        IcpRefillStoreOps::validate_retry_request_matches_operation(
            &request,
            root_canister,
            &operation,
        )?;
        execute_automatic_request(request, root_canister, sequence, None).await
    }

    /// Start one fresh automatic refill after the timer proves Coordinator fallback eligibility.
    pub(crate) async fn start_automatic_refill(
        current_cycles: u128,
        coordinator_operation_id: [u8; 32],
    ) -> Result<IcpRefillResponse, InternalError> {
        let root_canister = IcOps::canister_self();
        let prepared =
            prepare_automatic_refill(current_cycles, coordinator_operation_id, root_canister)
                .await?;
        execute_automatic_request(
            prepared.request,
            root_canister,
            prepared.sequence,
            Some(prepared.context),
        )
        .await
    }
}

async fn execute_automatic_request(
    request: IcpRefillRequest,
    root_canister: Principal,
    sequence: u64,
    context: Option<IcpRefillExecutionContext>,
) -> Result<IcpRefillResponse, InternalError> {
    let replay_input =
        icp_refill_replay_reserve_input(&request, root_canister, root_canister, IcOps::now_nanos());
    match reserve_icp_refill_replay(replay_input)? {
        IcpRefillReplayReservation::Fresh {
            operation_id,
            token,
        } => {
            log_icp_refill_fresh_reservation(&request, root_canister);
            execute_fresh_automatic_refill(
                request,
                operation_id,
                root_canister,
                sequence,
                context,
                &token,
            )
            .await
        }
        IcpRefillReplayReservation::Replay(response) => {
            log_icp_refill_committed_replay(&response);
            Ok(response)
        }
    }
}

async fn prepare_automatic_refill(
    current_cycles: u128,
    coordinator_operation_id: [u8; 32],
    root_canister: Principal,
) -> Result<PreparedAutomaticRefill, InternalError> {
    let policy = current_icp_refill_policy()?;
    validate_icp_refill_configured(policy.as_ref())?;
    let automatic = policy
        .as_ref()
        .and_then(|policy| policy.automatic.as_ref())
        .ok_or_else(|| super::policy_denied(IcpRefillPolicyViolation::NotConfigured))?;
    let emergency_threshold = automatic.emergency_threshold.to_u128();
    validate_automatic_refill_trigger(current_cycles, emergency_threshold)?;

    let policy_hash = current_root_funding_policy_hash()?;
    let canisters = IcpRefillOps::resolve_canisters(
        require_build_network(BuildNetworkOps::build_network())?,
        refill_canister_overrides(policy.as_ref()),
    )?;
    let fee_e8s = checked_nat_u64(
        "icrc1_fee",
        IcpRefillOps::icrc1_fee(canisters.ledger_canister_id).await?,
    )?;
    let decimals = IcpRefillOps::icrc1_decimals(canisters.ledger_canister_id).await?;
    validate_ledger_decimals(decimals)?;
    debug_assert_eq!(decimals, ICP_LEDGER_DECIMALS);
    let source_balance_e8s = checked_nat_u64(
        "icrc1_balance_of",
        IcpRefillOps::icrc1_balance_of(
            canisters.ledger_canister_id,
            Icrc1Account {
                owner: root_canister,
                subaccount: None,
            },
        )
        .await?,
    )?;
    let rate = super::configured_rate(
        policy.as_ref(),
        canisters.cmc_canister_id,
        RateQueryMode::Always,
    )
    .await?
    .ok_or_else(InternalError::invariant)?;

    let current_policy = current_icp_refill_policy()?;
    if current_policy != policy || current_root_funding_policy_hash()? != policy_hash {
        return Err(InternalError::public(
            crate::diagnostics::codes::STATE_CONFLICT,
        ));
    }
    let policy = current_policy.ok_or_else(InternalError::invariant)?;
    let automatic = policy
        .automatic
        .as_ref()
        .ok_or_else(InternalError::invariant)?;
    let amount_e8s =
        automatic_refill_amount_e8s(current_cycles, automatic.target_balance.to_u128(), rate)
            .map_err(IcpRefillWorkflowError::AutomaticAmount)?;
    let window_start_secs = fixed_window_start(IcOps::now_secs(), policy.window_secs);
    let usage = IcpRefillStoreOps::policy_usage(window_start_secs);
    let sequence = IcpRefillStoreOps::next_automatic_sequence()?;
    let operation_id = automatic_operation_id(
        root_canister,
        coordinator_operation_id,
        sequence,
        policy_hash,
        amount_e8s,
    );
    let request = IcpRefillRequest {
        operation_id,
        source_subaccount: None,
        amount_e8s,
        dry_run: false,
    };
    let mut preflight = RefillPreflight::new(Some(&policy), &request, root_canister)?;
    preflight.input.observed_fee_e8s = Some(fee_e8s);
    preflight.input.observed_source_balance_e8s = Some(source_balance_e8s);
    preflight.input.window_reserved_e8s = usage.window_reserved_e8s;
    preflight.evaluate(IcpRefillTrigger::Automatic { sequence }, Some(rate))?;

    Ok(PreparedAutomaticRefill {
        request,
        sequence,
        context: IcpRefillExecutionContext {
            ledger_canister_id: canisters.ledger_canister_id,
            cmc_canister_id: canisters.cmc_canister_id,
            fee_e8s,
            xdr_permyriad_per_icp: Some(rate),
            budget_window_start_secs: window_start_secs,
            policy_hash,
            created_at_time_ns: IcOps::now_nanos(),
        },
    })
}

fn validate_automatic_refill_trigger(
    current_cycles: u128,
    emergency_threshold: u128,
) -> Result<(), InternalError> {
    if current_cycles > emergency_threshold {
        return Err(IcpRefillWorkflowError::AutomaticThresholdNotMet {
            current_cycles,
            emergency_threshold,
        }
        .into());
    }
    if !FleetStateOps::cycles_funding_enabled() {
        return Err(super::policy_denied(
            IcpRefillPolicyViolation::CyclesFundingDisabled,
        ));
    }
    if IcpRefillStoreOps::active_operation()?.is_some() {
        return Err(super::policy_denied(
            IcpRefillPolicyViolation::ConcurrentRefill,
        ));
    }
    Ok(())
}

fn automatic_operation_id(
    root_canister: Principal,
    coordinator_operation_id: [u8; 32],
    sequence: u64,
    policy_hash: [u8; 32],
    amount_e8s: u64,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_part(&mut hasher, AUTOMATIC_OPERATION_ID_DOMAIN);
    hash_part(&mut hasher, root_canister.as_slice());
    hash_part(&mut hasher, &coordinator_operation_id);
    hash_part(&mut hasher, &sequence.to_be_bytes());
    hash_part(&mut hasher, &policy_hash);
    hash_part(&mut hasher, &amount_e8s.to_be_bytes());
    hasher.finalize().into()
}

fn hash_part(hasher: &mut Sha256, value: &[u8]) {
    hasher.update(u64::try_from(value.len()).unwrap_or(u64::MAX).to_be_bytes());
    hasher.update(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_operation_identity_binds_trigger_and_exact_amount() {
        let root = Principal::from_slice(&[1; 29]);
        let operation = automatic_operation_id(root, [2; 32], 3, [4; 32], 5);

        assert_eq!(
            operation,
            automatic_operation_id(root, [2; 32], 3, [4; 32], 5)
        );
        assert_ne!(
            operation,
            automatic_operation_id(root, [9; 32], 3, [4; 32], 5)
        );
        assert_ne!(
            operation,
            automatic_operation_id(root, [2; 32], 4, [4; 32], 5)
        );
        assert_ne!(
            operation,
            automatic_operation_id(root, [2; 32], 3, [9; 32], 5)
        );
        assert_ne!(
            operation,
            automatic_operation_id(root, [2; 32], 3, [4; 32], 6)
        );
    }
}
