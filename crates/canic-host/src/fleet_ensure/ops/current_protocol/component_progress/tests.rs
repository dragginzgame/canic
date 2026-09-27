//! Names and observed member progress remain distinct from Root aggregate counts.

use super::*;
use canic_core::dto::component_provisioning::{
    FleetComponentActivationRootProgress, FleetComponentProvisioningRootProgress,
};

pub(in crate::fleet_ensure::ops::current_protocol) fn qualify(
    plan: &FleetComponentProvisioningPlan,
    status: &FleetComponentProvisioningStatusResponse,
) {
    let original = project(plan, status);
    assert_eq!(original.len(), 2);
    assert_ne!(original[0].root, original[1].root);
    assert!(
        original
            .iter()
            .all(|row| row.state == State::Unknown && !row.current)
    );
    assert_eq!(original[0].component_spec, "alpha");
    assert_eq!(original[0].member_path, ["alpha"]);
    assert_ne!(original[0].placement, original[1].placement);

    let mut plan = plan.clone();
    let second = plan.batches.remove(1);
    plan.batches[0].placements.extend(second.placements);
    let root = plan.batches[0].root.fleet_subnet_root;
    let mut status = status.clone();
    status.current_root = Some(FleetComponentProvisioningRootProgress {
        fleet_subnet_root: root,
        component_count: 2,
        reserved_component_count: 2,
        claimed_component_count: 2,
        installed_component_count: 1,
        registry_committed_component_count: 0,
    });
    let rows = project(&plan, &status);
    assert_eq!(rows[0].state, State::Installed);
    assert_eq!(rows[1].state, State::Claimed);
    assert!(!rows[0].current);
    assert!(rows[1].current);

    status
        .current_root
        .as_mut()
        .unwrap()
        .registry_committed_component_count = 2;
    assert!(
        project(&plan, &status)
            .iter()
            .all(|row| row.state == State::Unknown)
    );
    status.current_root = None;
    status.current_publication = Some(
        canic_core::dto::component_provisioning::FleetComponentPublicationRootProgress {
            fleet_subnet_root: root,
            component_count: 2,
            published_component_count: 1,
        },
    );
    let rows = project(&plan, &status);
    assert_eq!(rows[0].state, State::Published);
    assert_eq!(rows[1].state, State::Registered);
    assert!(rows[1].current);
    status.current_publication = None;
    status.current_activation = Some(FleetComponentActivationRootProgress {
        fleet_subnet_root: root,
        component_count: 2,
        activated_component_count: 1,
        root_runtime_active: false,
    });
    let rows = project(&plan, &status);
    assert_eq!(rows[0].state, State::Active);
    assert_eq!(rows[1].state, State::RuntimePending);
    assert!(rows[1].current);
    status
        .current_activation
        .as_mut()
        .unwrap()
        .fleet_subnet_root = Principal::anonymous();
    assert!(
        project(&plan, &status)
            .iter()
            .all(|row| row.state == State::Unknown)
    );
}
