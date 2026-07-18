//! Module: plan::preflight::requests
//!
//! Responsibility: project backup plans into typed execution preflight requests.
//! Does not own: request execution, receipt validation, or plan construction.
//! Boundary: exposes immutable request views for external preflight providers.

use crate::plan::{
    BackupPlan, ControlAuthorityPreflightRequest, ControlAuthorityPreflightTarget,
    QuiescencePreflightRequest, QuiescencePreflightTarget, SnapshotReadAuthorityPreflightRequest,
    SnapshotReadAuthorityPreflightTarget, TopologyPreflightRequest, TopologyPreflightTarget,
};

impl BackupPlan {
    /// Build the typed control-authority preflight request for this plan.
    #[must_use]
    pub fn control_authority_preflight_request(&self) -> ControlAuthorityPreflightRequest {
        ControlAuthorityPreflightRequest {
            plan_id: self.plan_id.clone(),
            run_id: self.run_id.clone(),
            fleet: self.fleet.clone(),
            environment: self.environment.clone(),
            root_canister_id: self.root_canister_id.clone(),
            requires_root_controller: self.requires_root_controller,
            targets: self
                .targets
                .iter()
                .map(ControlAuthorityPreflightTarget::from)
                .collect(),
        }
    }

    /// Build the typed snapshot-read preflight request for this plan.
    #[must_use]
    pub fn snapshot_read_authority_preflight_request(
        &self,
    ) -> SnapshotReadAuthorityPreflightRequest {
        SnapshotReadAuthorityPreflightRequest {
            plan_id: self.plan_id.clone(),
            run_id: self.run_id.clone(),
            fleet: self.fleet.clone(),
            environment: self.environment.clone(),
            root_canister_id: self.root_canister_id.clone(),
            targets: self
                .targets
                .iter()
                .map(SnapshotReadAuthorityPreflightTarget::from)
                .collect(),
        }
    }

    /// Build the typed topology preflight request for this plan.
    #[must_use]
    pub fn topology_preflight_request(&self) -> TopologyPreflightRequest {
        TopologyPreflightRequest {
            plan_id: self.plan_id.clone(),
            run_id: self.run_id.clone(),
            fleet: self.fleet.clone(),
            environment: self.environment.clone(),
            root_canister_id: self.root_canister_id.clone(),
            selected_subtree_root: self.selected_subtree_root.clone(),
            selected_scope_kind: self.selected_scope_kind.clone(),
            topology_hash_before_quiesce: self.topology_hash_before_quiesce.clone(),
            targets: self
                .targets
                .iter()
                .map(TopologyPreflightTarget::from)
                .collect(),
        }
    }

    /// Build the typed quiescence preflight request for this plan.
    #[must_use]
    pub fn quiescence_preflight_request(&self) -> QuiescencePreflightRequest {
        QuiescencePreflightRequest {
            plan_id: self.plan_id.clone(),
            run_id: self.run_id.clone(),
            fleet: self.fleet.clone(),
            environment: self.environment.clone(),
            root_canister_id: self.root_canister_id.clone(),
            selected_subtree_root: self.selected_subtree_root.clone(),
            quiescence_policy: self.quiescence_policy.clone(),
            targets: self
                .targets
                .iter()
                .map(QuiescencePreflightTarget::from)
                .collect(),
        }
    }
}
