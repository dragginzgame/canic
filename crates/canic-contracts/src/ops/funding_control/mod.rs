//! Lossless funding boundary conversions. No policy, storage or spending changes.
use crate::{
    dto::{fleet_funding as stored, funding_control as wire},
    ids::SubnetId,
};
impl From<stored::FleetFundingPolicyRotationPlacementEvidence> for wire::PlacementEvidence {
    fn from(value: stored::FleetFundingPolicyRotationPlacementEvidence) -> Self {
        Self {
            subnet: value.subnet.into_principal().into(),
            node_count: value.node_count,
            cost_multiplier_numerator: value.cost_multiplier_numerator,
            cost_multiplier_denominator: value.cost_multiplier_denominator,
            fiduciary: value.fiduciary,
            acknowledge_fiduciary_cost: value.acknowledge_fiduciary_cost,
        }
    }
}
impl From<wire::PlacementEvidence> for stored::FleetFundingPolicyRotationPlacementEvidence {
    fn from(value: wire::PlacementEvidence) -> Self {
        Self {
            subnet: SubnetId::from_principal(value.subnet.into()),
            node_count: value.node_count,
            cost_multiplier_numerator: value.cost_multiplier_numerator,
            cost_multiplier_denominator: value.cost_multiplier_denominator,
            fiduciary: value.fiduciary,
            acknowledge_fiduciary_cost: value.acknowledge_fiduciary_cost,
        }
    }
}
impl From<stored::FleetFundingPolicyRotationPlanHeader> for wire::PlanHeader {
    fn from(value: stored::FleetFundingPolicyRotationPlanHeader) -> Self {
        Self {
            predecessor_registry: value.predecessor_registry.into(),
            predecessor_generation: value.predecessor_generation,
            successor_generation: value.successor_generation,
            predecessor_coordinator_policy_hash: value.predecessor_coordinator_policy_hash,
            predecessor_usage: value.predecessor_usage,
            proposed_coordinator_policy: value.proposed_coordinator_policy,
            topology_catalog_digest: value.topology_catalog_digest,
            coordinator_placement: value.coordinator_placement.into(),
            affected_root_count: value.affected_root_count,
            roots_digest: value.roots_digest,
            maximum_new_automatic_cycles: value.maximum_new_automatic_cycles,
            apply_operator_debit: value.apply_operator_debit,
            funding_source: value.funding_source,
        }
    }
}
impl From<wire::PlanHeader> for stored::FleetFundingPolicyRotationPlanHeader {
    fn from(value: wire::PlanHeader) -> Self {
        Self {
            predecessor_registry: value.predecessor_registry.into(),
            predecessor_generation: value.predecessor_generation,
            successor_generation: value.successor_generation,
            predecessor_coordinator_policy_hash: value.predecessor_coordinator_policy_hash,
            predecessor_usage: value.predecessor_usage,
            proposed_coordinator_policy: value.proposed_coordinator_policy,
            topology_catalog_digest: value.topology_catalog_digest,
            coordinator_placement: value.coordinator_placement.into(),
            affected_root_count: value.affected_root_count,
            roots_digest: value.roots_digest,
            maximum_new_automatic_cycles: value.maximum_new_automatic_cycles,
            apply_operator_debit: value.apply_operator_debit,
            funding_source: value.funding_source,
        }
    }
}
impl From<stored::FleetFundingPolicyRotationRootPlan> for wire::RootPlan {
    fn from(value: stored::FleetFundingPolicyRotationRootPlan) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            predecessor_policy_hash: value.predecessor_policy_hash,
            predecessor_usage: value.predecessor_usage,
            proposed_policy: value.proposed_policy,
            placement: value.placement.into(),
        }
    }
}
impl From<wire::RootPlan> for stored::FleetFundingPolicyRotationRootPlan {
    fn from(value: wire::RootPlan) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            predecessor_policy_hash: value.predecessor_policy_hash,
            predecessor_usage: value.predecessor_usage,
            proposed_policy: value.proposed_policy,
            placement: value.placement.into(),
        }
    }
}
impl From<stored::FleetFundingPolicyRotationBeginRequest> for wire::BeginRequest {
    fn from(value: stored::FleetFundingPolicyRotationBeginRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            header: value.header.into(),
        }
    }
}
impl From<wire::BeginRequest> for stored::FleetFundingPolicyRotationBeginRequest {
    fn from(value: wire::BeginRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            header: value.header.into(),
        }
    }
}
impl From<stored::FleetFundingPolicyRotationStageRootRequest> for wire::StageRequest {
    fn from(value: stored::FleetFundingPolicyRotationStageRootRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            root: value.root.into(),
        }
    }
}
impl From<wire::StageRequest> for stored::FleetFundingPolicyRotationStageRootRequest {
    fn from(value: wire::StageRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            root: value.root.into(),
        }
    }
}
impl From<stored::FleetFundingPolicyRotationRootPrepareRequest> for wire::PrepareRequest {
    fn from(value: stored::FleetFundingPolicyRotationRootPrepareRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            predecessor_registry: value.predecessor_registry.into(),
            predecessor_generation: value.predecessor_generation,
            successor_generation: value.successor_generation,
            root: value.root.into(),
        }
    }
}
impl From<wire::PrepareRequest> for stored::FleetFundingPolicyRotationRootPrepareRequest {
    fn from(value: wire::PrepareRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            predecessor_registry: value.predecessor_registry.into(),
            predecessor_generation: value.predecessor_generation,
            successor_generation: value.successor_generation,
            root: value.root.into(),
        }
    }
}
impl From<stored::FleetFundingPolicyRotationRootActivateRequest> for wire::ActivateRequest {
    fn from(value: stored::FleetFundingPolicyRotationRootActivateRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            predecessor_registry: value.predecessor_registry.into(),
            successor_registry: value.successor_registry.into(),
            predecessor_generation: value.predecessor_generation,
            successor_generation: value.successor_generation,
            fleet_subnet_root: value.fleet_subnet_root.into(),
        }
    }
}
impl From<wire::ActivateRequest> for stored::FleetFundingPolicyRotationRootActivateRequest {
    fn from(value: wire::ActivateRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            predecessor_registry: value.predecessor_registry.into(),
            successor_registry: value.successor_registry.into(),
            predecessor_generation: value.predecessor_generation,
            successor_generation: value.successor_generation,
            fleet_subnet_root: value.fleet_subnet_root.into(),
        }
    }
}
impl From<stored::FleetFundingPolicyRotationRootReceipt> for wire::RotationReceipt {
    fn from(value: stored::FleetFundingPolicyRotationRootReceipt) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            predecessor_generation: value.predecessor_generation,
            successor_generation: value.successor_generation,
            prepared: value.prepared,
            activated: value.activated,
            recorded_at_ns: value.recorded_at_ns,
        }
    }
}
impl From<wire::RotationReceipt> for stored::FleetFundingPolicyRotationRootReceipt {
    fn from(value: wire::RotationReceipt) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_digest: value.plan_digest,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            predecessor_generation: value.predecessor_generation,
            successor_generation: value.successor_generation,
            prepared: value.prepared,
            activated: value.activated,
            recorded_at_ns: value.recorded_at_ns,
        }
    }
}
impl From<stored::FleetRootFundingRequest> for wire::Request {
    fn from(value: stored::FleetRootFundingRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            operation_sequence: value.operation_sequence,
            expected_registry: value.expected_registry.into(),
            observed_balance: value.observed_balance,
            requested_cycles: value.requested_cycles,
            policy_hash: value.policy_hash,
        }
    }
}
impl From<wire::Request> for stored::FleetRootFundingRequest {
    fn from(value: wire::Request) -> Self {
        Self {
            operation_id: value.operation_id,
            operation_sequence: value.operation_sequence,
            expected_registry: value.expected_registry.into(),
            observed_balance: value.observed_balance,
            requested_cycles: value.requested_cycles,
            policy_hash: value.policy_hash,
        }
    }
}
impl From<stored::FleetRootFundingAcceptanceRequest> for wire::AcceptanceRequest {
    fn from(value: stored::FleetRootFundingAcceptanceRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            operation_sequence: value.operation_sequence,
            expected_registry: value.expected_registry.into(),
            observed_balance: value.observed_balance,
            granted_cycles: value.granted_cycles,
            policy_hash: value.policy_hash,
        }
    }
}
impl From<wire::AcceptanceRequest> for stored::FleetRootFundingAcceptanceRequest {
    fn from(value: wire::AcceptanceRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            operation_sequence: value.operation_sequence,
            expected_registry: value.expected_registry.into(),
            observed_balance: value.observed_balance,
            granted_cycles: value.granted_cycles,
            policy_hash: value.policy_hash,
        }
    }
}
impl From<stored::FleetRootFundingAcceptanceReceipt> for wire::AcceptanceReceipt {
    fn from(value: stored::FleetRootFundingAcceptanceReceipt) -> Self {
        Self {
            request: value.request.into(),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            coordinator: value.coordinator.into(),
            accepted_at_ns: value.accepted_at_ns,
        }
    }
}
impl From<wire::AcceptanceReceipt> for stored::FleetRootFundingAcceptanceReceipt {
    fn from(value: wire::AcceptanceReceipt) -> Self {
        Self {
            request: value.request.into(),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            coordinator: value.coordinator.into(),
            accepted_at_ns: value.accepted_at_ns,
        }
    }
}
impl From<stored::FleetRootFundingNoGrantReceipt> for wire::NoGrantReceipt {
    fn from(value: stored::FleetRootFundingNoGrantReceipt) -> Self {
        Self {
            request: value.request.into(),
            reason: value.reason,
            decided_at_ns: value.decided_at_ns,
        }
    }
}
impl From<wire::NoGrantReceipt> for stored::FleetRootFundingNoGrantReceipt {
    fn from(value: wire::NoGrantReceipt) -> Self {
        Self {
            request: value.request.into(),
            reason: value.reason,
            decided_at_ns: value.decided_at_ns,
        }
    }
}
impl From<stored::FleetRootFundingResponse> for wire::Response {
    fn from(value: stored::FleetRootFundingResponse) -> Self {
        match value {
            stored::FleetRootFundingResponse::Granted(v) => Self::Granted(v.into()),
            stored::FleetRootFundingResponse::NoGrant(v) => Self::NoGrant(v.into()),
        }
    }
}
impl From<wire::Response> for stored::FleetRootFundingResponse {
    fn from(value: wire::Response) -> Self {
        match value {
            wire::Response::Granted(v) => Self::Granted(v.into()),
            wire::Response::NoGrant(v) => Self::NoGrant(v.into()),
        }
    }
}
