//! Module: workflow::ic::icp_refill::hub
//!
//! Responsibility: orchestrate automatic hub self-refill requests.
//! Does not own: refill policy evaluation, storage mutation, or IC ledger calls.
//! Boundary: builds self-refill requests and delegates execution to manual refill flow.

use crate::{
    InternalError,
    cdk::types::{Cycles, Principal},
    domain::policy::icp_refill::{IcpRefillPolicyViolation, evaluate_hub_self_refill},
    dto::icp_refill::{IcpRefillMode, IcpRefillRequest, IcpRefillResponse},
    ops::{
        ic::{IcOps, icp_refill::IcpRefillOps},
        storage::{icp_refill::IcpRefillStoreOps, state::app::AppStateOps},
    },
    workflow::ic::icp_refill::{
        IcpRefillWorkflow, RateQueryMode, build_network, configured_rate, current_topup_policy,
        funding_cooldown_retry_after_secs, icp_refill_policy_rules, in_flight_for_request,
        policy_denied, policy_input, refill_canister_overrides,
    },
};
use sha2::{Digest, Sha256};

impl IcpRefillWorkflow {
    pub async fn execute_hub_self_refill(
        hub_cycles: Cycles,
    ) -> Result<IcpRefillResponse, InternalError> {
        let self_pid = IcOps::canister_self();
        if let Some(operation) = IcpRefillStoreOps::find_resumable_hub_self_refill(self_pid) {
            let request = IcpRefillStoreOps::to_request(&operation);
            return Self::execute_manual_refill(request).await;
        }

        let Some(topup) = current_topup_policy()? else {
            return Err(policy_denied(IcpRefillPolicyViolation::NotConfigured));
        };
        let Some(icp_refill) = topup.icp_refill.as_ref() else {
            return Err(policy_denied(IcpRefillPolicyViolation::NotConfigured));
        };
        let canisters = IcpRefillOps::resolve_canisters(
            build_network(),
            refill_canister_overrides(Some(icp_refill)),
        )?;
        let observed_rate = configured_rate(
            Some(icp_refill),
            canisters.cmc_canister_id,
            RateQueryMode::WhenGateConfigured,
        )
        .await?;
        let now_ns = IcOps::now_nanos();
        let now_secs = IcOps::now_secs();
        let request = IcpRefillRequest {
            operation_id: hub_self_refill_operation_id(
                self_pid,
                None,
                self_pid,
                icp_refill.max_refill_e8s_per_call,
                now_ns,
            ),
            source_canister: self_pid,
            source_subaccount: None,
            target_canister: self_pid,
            amount_e8s: icp_refill.max_refill_e8s_per_call,
            dry_run: false,
            mode: IcpRefillMode::Canister,
        };

        let policy_rules = icp_refill_policy_rules(icp_refill);
        evaluate_hub_self_refill(
            Some(&policy_rules),
            policy_input(
                hub_cycles.to_u128(),
                &request,
                observed_rate,
                in_flight_for_request(&request),
                AppStateOps::cycles_funding_enabled(),
                funding_cooldown_retry_after_secs(&request, now_secs),
            ),
        )
        .map_err(policy_denied)?;

        Self::execute_manual_refill(request).await
    }
}

pub(super) fn hub_self_refill_operation_id(
    source_canister: Principal,
    source_subaccount: Option<[u8; 32]>,
    target_canister: Principal,
    amount_e8s: u64,
    now_ns: u64,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"canic:icp-refill:hub-self-refill:v1");
    hasher.update(source_canister.as_slice());
    hasher.update(source_subaccount.unwrap_or_default());
    hasher.update(target_canister.as_slice());
    hasher.update(amount_e8s.to_be_bytes());
    hasher.update(now_ns.to_be_bytes());
    hasher.finalize().into()
}
