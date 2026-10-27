//! Fallible, lossless conversion of external numeric evidence at the funding boundary.
use crate::{
    dto::{funding_read as wire, icp_refill, root},
    ops::unsigned_magnitude,
};
impl TryFrom<icp_refill::IcpRefillResponse> for wire::RefillResult {
    type Error = ic_wire::Error;
    fn try_from(value: icp_refill::IcpRefillResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            operation_id: value.operation_id,
            status: value.status,
            ledger_block_index: value.ledger_block_index,
            cycles_sent: value
                .cycles_sent
                .as_ref()
                .map(unsigned_magnitude::from_nat)
                .transpose()?,
            error_code: value.error_code,
            error_message: value.error_message,
        })
    }
}
impl TryFrom<wire::RefillResult> for icp_refill::IcpRefillResponse {
    type Error = ic_wire::Error;
    fn try_from(value: wire::RefillResult) -> Result<Self, Self::Error> {
        Ok(Self {
            operation_id: value.operation_id,
            status: value.status,
            ledger_block_index: value.ledger_block_index,
            cycles_sent: value
                .cycles_sent
                .as_ref()
                .map(unsigned_magnitude::to_nat)
                .transpose()?,
            error_code: value.error_code,
            error_message: value.error_message,
        })
    }
}
impl TryFrom<root::RootIcpRefillReleaseEvidence> for wire::RefillEvidence {
    type Error = ic_wire::Error;
    fn try_from(value: root::RootIcpRefillReleaseEvidence) -> Result<Self, Self::Error> {
        Ok(Self {
            transfer_uncertain: value.transfer_uncertain,
            record_id: value.record_id,
            trigger: value.trigger,
            policy_hash: value.policy_hash,
            source_canister: value.source_canister.into(),
            source_subaccount: value.source_subaccount,
            target_canister: value.target_canister.into(),
            ledger_canister_id: value.ledger_canister_id.into(),
            cmc_canister_id: value.cmc_canister_id.into(),
            cmc_to_account_owner: value.cmc_to_account_owner.into(),
            cmc_to_account_subaccount: value.cmc_to_account_subaccount,
            amount_e8s: value.amount_e8s,
            fee_e8s: value.fee_e8s,
            budget_window_start_secs: value.budget_window_start_secs,
            budget_reserved: value.budget_reserved,
            memo: value.memo,
            created_at_time_ns: value.created_at_time_ns,
            notify_attempts: value.notify_attempts,
            response: value.response.try_into()?,
            refund_block_index: value.refund_block_index,
            transaction_too_old_min_block_index: value.transaction_too_old_min_block_index,
        })
    }
}
impl TryFrom<wire::RefillEvidence> for root::RootIcpRefillReleaseEvidence {
    type Error = ic_wire::Error;
    fn try_from(value: wire::RefillEvidence) -> Result<Self, Self::Error> {
        Ok(Self {
            transfer_uncertain: value.transfer_uncertain,
            record_id: value.record_id,
            trigger: value.trigger,
            policy_hash: value.policy_hash,
            source_canister: value.source_canister.into(),
            source_subaccount: value.source_subaccount,
            target_canister: value.target_canister.into(),
            ledger_canister_id: value.ledger_canister_id.into(),
            cmc_canister_id: value.cmc_canister_id.into(),
            cmc_to_account_owner: value.cmc_to_account_owner.into(),
            cmc_to_account_subaccount: value.cmc_to_account_subaccount,
            amount_e8s: value.amount_e8s,
            fee_e8s: value.fee_e8s,
            budget_window_start_secs: value.budget_window_start_secs,
            budget_reserved: value.budget_reserved,
            memo: value.memo,
            created_at_time_ns: value.created_at_time_ns,
            notify_attempts: value.notify_attempts,
            response: value.response.try_into()?,
            refund_block_index: value.refund_block_index,
            transaction_too_old_min_block_index: value.transaction_too_old_min_block_index,
        })
    }
}
impl TryFrom<root::RootFundingReleaseResponse> for wire::ReleasePage {
    type Error = ic_wire::Error;
    fn try_from(value: root::RootFundingReleaseResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            policy_generation: value.policy_generation,
            policy_hash: value.policy_hash,
            icp_refill_policy: value.icp_refill_policy.map(Into::into),
            current_request: value.current_request.map(Into::into),
            accepted_grant: value.accepted_grant.map(Into::into),
            rotation_current: value.rotation_current.map(Into::into),
            icp_refills: value
                .icp_refills
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            next_after: value.next_after,
        })
    }
}
impl TryFrom<wire::ReleasePage> for root::RootFundingReleaseResponse {
    type Error = ic_wire::Error;
    fn try_from(value: wire::ReleasePage) -> Result<Self, Self::Error> {
        Ok(Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            policy_generation: value.policy_generation,
            policy_hash: value.policy_hash,
            icp_refill_policy: value.icp_refill_policy.map(Into::into),
            current_request: value.current_request.map(Into::into),
            accepted_grant: value.accepted_grant.map(Into::into),
            rotation_current: value.rotation_current.map(Into::into),
            icp_refills: value
                .icp_refills
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            next_after: value.next_after,
        })
    }
}

impl TryFrom<root::RootIcpRefillStatusResponse> for wire::RefillStatus {
    type Error = ic_wire::Error;
    fn try_from(value: root::RootIcpRefillStatusResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            trigger: value.trigger,
            amount_e8s: value.amount_e8s,
            fee_e8s: value.fee_e8s,
            budget_window_start_secs: value.budget_window_start_secs,
            resumable: value.resumable,
            response: value.response.try_into()?,
        })
    }
}

impl TryFrom<wire::RefillStatus> for root::RootIcpRefillStatusResponse {
    type Error = ic_wire::Error;
    fn try_from(value: wire::RefillStatus) -> Result<Self, Self::Error> {
        Ok(Self {
            trigger: value.trigger,
            amount_e8s: value.amount_e8s,
            fee_e8s: value.fee_e8s,
            budget_window_start_secs: value.budget_window_start_secs,
            resumable: value.resumable,
            response: value.response.try_into()?,
        })
    }
}

impl TryFrom<root::RootFundingStatusResponse> for wire::RootStatus {
    type Error = ic_wire::Error;
    fn try_from(value: root::RootFundingStatusResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            lifecycle_status: value.lifecycle_status,
            funding_eligible: value.funding_eligible,
            cycles_funding_enabled: value.cycles_funding_enabled,
            current_cycles: value.current_cycles,
            policy_generation: value.policy_generation,
            funding_profile: value.funding_profile,
            policy_hash: value.policy_hash,
            root_policy: value.root_policy,
            current_operation: value.current_operation.map(Into::into),
            last_result: value.last_result.map(Into::into),
            historical_automatic_grants: value.historical_automatic_grants,
            historical_automatic_cycles: value.historical_automatic_cycles,
            automatic_grants: value.automatic_grants,
            automatic_cycles: value.automatic_cycles,
            rotation_current: value.rotation_current.map(Into::into),
            rotation_last: value.rotation_last.map(Into::into),
            icp_refill_policy: value.icp_refill_policy.map(Into::into),
            icp_window_start_secs: value.icp_window_start_secs,
            icp_window_reserved_e8s: value.icp_window_reserved_e8s,
            automatic_icp_refills: value.automatic_icp_refills,
            automatic_icp_refill_e8s: value.automatic_icp_refill_e8s,
            latest_icp_refill: value.latest_icp_refill.map(TryInto::try_into).transpose()?,
        })
    }
}

impl TryFrom<wire::RootStatus> for root::RootFundingStatusResponse {
    type Error = ic_wire::Error;
    fn try_from(value: wire::RootStatus) -> Result<Self, Self::Error> {
        Ok(Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            lifecycle_status: value.lifecycle_status,
            funding_eligible: value.funding_eligible,
            cycles_funding_enabled: value.cycles_funding_enabled,
            current_cycles: value.current_cycles,
            policy_generation: value.policy_generation,
            funding_profile: value.funding_profile,
            policy_hash: value.policy_hash,
            root_policy: value.root_policy,
            current_operation: value.current_operation.map(Into::into),
            last_result: value.last_result.map(Into::into),
            historical_automatic_grants: value.historical_automatic_grants,
            historical_automatic_cycles: value.historical_automatic_cycles,
            automatic_grants: value.automatic_grants,
            automatic_cycles: value.automatic_cycles,
            rotation_current: value.rotation_current.map(Into::into),
            rotation_last: value.rotation_last.map(Into::into),
            icp_refill_policy: value.icp_refill_policy.map(Into::into),
            icp_window_start_secs: value.icp_window_start_secs,
            icp_window_reserved_e8s: value.icp_window_reserved_e8s,
            automatic_icp_refills: value.automatic_icp_refills,
            automatic_icp_refill_e8s: value.automatic_icp_refill_e8s,
            latest_icp_refill: value.latest_icp_refill.map(TryInto::try_into).transpose()?,
        })
    }
}

impl From<crate::dto::fleet_coordinator::CoordinatorRootFundingStatusResponse>
    for wire::CoordinatorRoot
{
    fn from(value: crate::dto::fleet_coordinator::CoordinatorRootFundingStatusResponse) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            lifecycle_status: value.lifecycle_status,
            policy_hash: value.policy_hash,
            policy: value.policy,
            window: value.window,
            historical_automatic_grants: value.historical_automatic_grants,
            historical_automatic_cycles: value.historical_automatic_cycles,
            automatic_grants: value.automatic_grants,
            automatic_cycles: value.automatic_cycles,
            last_successful_grant_at_ns: value.last_successful_grant_at_ns,
            current_operation: value.current_operation.map(Into::into),
            last_result: value.last_result.map(Into::into),
        }
    }
}

impl From<wire::CoordinatorRoot>
    for crate::dto::fleet_coordinator::CoordinatorRootFundingStatusResponse
{
    fn from(value: wire::CoordinatorRoot) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            lifecycle_status: value.lifecycle_status,
            policy_hash: value.policy_hash,
            policy: value.policy,
            window: value.window,
            historical_automatic_grants: value.historical_automatic_grants,
            historical_automatic_cycles: value.historical_automatic_cycles,
            automatic_grants: value.automatic_grants,
            automatic_cycles: value.automatic_cycles,
            last_successful_grant_at_ns: value.last_successful_grant_at_ns,
            current_operation: value.current_operation.map(Into::into),
            last_result: value.last_result.map(Into::into),
        }
    }
}

impl From<crate::dto::fleet_coordinator::CoordinatorFundingStatusResponse>
    for wire::CoordinatorStatus
{
    fn from(value: crate::dto::fleet_coordinator::CoordinatorFundingStatusResponse) -> Self {
        Self {
            coordinator: value.coordinator.into(),
            current_cycles: value.current_cycles,
            policy_generation: value.policy_generation,
            funding_enabled: value.funding_enabled,
            funding_profile: value.funding_profile,
            policy: value.policy,
            fleet_window: value.fleet_window,
            historical_automatic_grants: value.historical_automatic_grants,
            historical_automatic_cycles: value.historical_automatic_cycles,
            automatic_grants: value.automatic_grants,
            automatic_cycles: value.automatic_cycles,
            rotation_checkpoint_count: value.rotation_checkpoint_count,
            rotation_checkpoint_root_count: value.rotation_checkpoint_root_count,
            rotation_checkpoint_root_capacity_remaining: value
                .rotation_checkpoint_root_capacity_remaining,
            rotation: value.rotation.map(Into::into),
            roots: value.roots.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<wire::CoordinatorStatus>
    for crate::dto::fleet_coordinator::CoordinatorFundingStatusResponse
{
    fn from(value: wire::CoordinatorStatus) -> Self {
        Self {
            coordinator: value.coordinator.into(),
            current_cycles: value.current_cycles,
            policy_generation: value.policy_generation,
            funding_enabled: value.funding_enabled,
            funding_profile: value.funding_profile,
            policy: value.policy,
            fleet_window: value.fleet_window,
            historical_automatic_grants: value.historical_automatic_grants,
            historical_automatic_cycles: value.historical_automatic_cycles,
            automatic_grants: value.automatic_grants,
            automatic_cycles: value.automatic_cycles,
            rotation_checkpoint_count: value.rotation_checkpoint_count,
            rotation_checkpoint_root_count: value.rotation_checkpoint_root_count,
            rotation_checkpoint_root_capacity_remaining: value
                .rotation_checkpoint_root_capacity_remaining,
            rotation: value.rotation.map(Into::into),
            roots: value.roots.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::dto::fleet_coordinator::FleetFundingPolicyRotationStatusResponse>
    for wire::RotationStatus
{
    fn from(
        value: crate::dto::fleet_coordinator::FleetFundingPolicyRotationStatusResponse,
    ) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            predecessor_generation: value.predecessor_generation,
            successor_generation: value.successor_generation,
            phase: value.phase.into(),
        }
    }
}

impl From<wire::RotationStatus>
    for crate::dto::fleet_coordinator::FleetFundingPolicyRotationStatusResponse
{
    fn from(value: wire::RotationStatus) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            predecessor_generation: value.predecessor_generation,
            successor_generation: value.successor_generation,
            phase: value.phase.into(),
        }
    }
}

impl From<crate::dto::fleet_funding::FleetFundingPolicyRotationReceipt>
    for wire::RotationCompletion
{
    fn from(value: crate::dto::fleet_funding::FleetFundingPolicyRotationReceipt) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            predecessor_registry: value.predecessor_registry.into(),
            successor_registry: value.successor_registry.into(),
            predecessor_generation: value.predecessor_generation,
            successor_generation: value.successor_generation,
            affected_root_count: value.affected_root_count,
            retained_historical_automatic_grants: value.retained_historical_automatic_grants,
            retained_historical_automatic_cycles: value.retained_historical_automatic_cycles,
            successor_policy_set_hash: value.successor_policy_set_hash,
            maximum_new_automatic_cycles: value.maximum_new_automatic_cycles,
            apply_operator_debit: value.apply_operator_debit,
            completed_at_ns: value.completed_at_ns,
        }
    }
}

impl From<wire::RotationCompletion>
    for crate::dto::fleet_funding::FleetFundingPolicyRotationReceipt
{
    fn from(value: wire::RotationCompletion) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            predecessor_registry: value.predecessor_registry.into(),
            successor_registry: value.successor_registry.into(),
            predecessor_generation: value.predecessor_generation,
            successor_generation: value.successor_generation,
            affected_root_count: value.affected_root_count,
            retained_historical_automatic_grants: value.retained_historical_automatic_grants,
            retained_historical_automatic_cycles: value.retained_historical_automatic_cycles,
            successor_policy_set_hash: value.successor_policy_set_hash,
            maximum_new_automatic_cycles: value.maximum_new_automatic_cycles,
            apply_operator_debit: value.apply_operator_debit,
            completed_at_ns: value.completed_at_ns,
        }
    }
}

impl From<crate::dto::fleet_coordinator::FleetFundingPolicyRotationStatusPhase>
    for wire::RotationPhase
{
    fn from(value: crate::dto::fleet_coordinator::FleetFundingPolicyRotationStatusPhase) -> Self {
        match value {
            crate::dto::fleet_coordinator::FleetFundingPolicyRotationStatusPhase::Staging { staged_root_count, expected_root_count } => Self::Staging { staged_root_count, expected_root_count },
            crate::dto::fleet_coordinator::FleetFundingPolicyRotationStatusPhase::PreparingRoots { prepared_root_count, expected_root_count } => Self::PreparingRoots { prepared_root_count, expected_root_count },
            crate::dto::fleet_coordinator::FleetFundingPolicyRotationStatusPhase::ActivatingRoots { activated_root_count, expected_root_count, successor_registry } => Self::ActivatingRoots { activated_root_count, expected_root_count, successor_registry: Box::new((*successor_registry).into()) },
            crate::dto::fleet_coordinator::FleetFundingPolicyRotationStatusPhase::Completed(receipt) => Self::Completed(Box::new((*receipt).into())),
        }
    }
}

impl From<wire::RotationPhase>
    for crate::dto::fleet_coordinator::FleetFundingPolicyRotationStatusPhase
{
    fn from(value: wire::RotationPhase) -> Self {
        match value {
            wire::RotationPhase::Staging {
                staged_root_count,
                expected_root_count,
            } => Self::Staging {
                staged_root_count,
                expected_root_count,
            },
            wire::RotationPhase::PreparingRoots {
                prepared_root_count,
                expected_root_count,
            } => Self::PreparingRoots {
                prepared_root_count,
                expected_root_count,
            },
            wire::RotationPhase::ActivatingRoots {
                activated_root_count,
                expected_root_count,
                successor_registry,
            } => Self::ActivatingRoots {
                activated_root_count,
                expected_root_count,
                successor_registry: Box::new((*successor_registry).into()),
            },
            wire::RotationPhase::Completed(receipt) => Self::Completed(Box::new((*receipt).into())),
        }
    }
}

#[cfg(test)]
mod tests;
