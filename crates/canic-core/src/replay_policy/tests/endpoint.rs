//! Module: replay_policy::tests::endpoint
//!
//! Responsibility: verify direct endpoint replay classifications.
//! Does not own: role-command variant policy or workflow replay behavior.
//! Boundary: focused assertions over the maintained endpoint manifest.

use super::*;

#[test]
fn checkpoint_observation_is_a_read_only_update_without_paid_effects() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_root_checkpoint_status")
        .unwrap();
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(entry.replay_policy, ReplayPolicy::QueryOrReadOnly);
    assert_eq!(entry.cost_class, CostClass::None);
    assert!(entry.quota_policy.is_none());
    assert!(entry.cycle_reserve_policy.is_none());
}

#[test]
fn native_inspection_and_role_queries_remain_read_only() {
    let inspection = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::inspection::METHOD)
        .unwrap();
    assert_eq!(inspection.endpoint_kind, EndpointKind::Update);
    assert_eq!(inspection.replay_policy, ReplayPolicy::QueryOrReadOnly);
    for endpoint in [
        crate::protocol::funding_read::coordinator_status::METHOD,
        crate::protocol::funding_read::root_status::METHOD,
    ] {
        let status = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == endpoint)
            .expect("infrastructure role status entry");
        assert_eq!(status.endpoint_kind, EndpointKind::Query);
        assert_eq!(status.replay_policy, ReplayPolicy::QueryOrReadOnly);
    }

    for method in [
        crate::protocol::store_status::authority::METHOD,
        crate::protocol::store_status::activation::METHOD,
        crate::protocol::store_status::gc::METHOD,
    ] {
        let entry = STORE_ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        assert_eq!(entry.endpoint_kind, EndpointKind::Query);
        assert_eq!(entry.replay_policy, ReplayPolicy::QueryOrReadOnly);
        assert_eq!(entry.cost_class, CostClass::None);
    }
}

#[test]
fn store_byte_lanes_keep_their_direct_replay_contracts() {
    let chunk = STORE_ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_wasm_store_chunk")
        .expect("Store chunk lane");
    assert_eq!(chunk.endpoint_kind, EndpointKind::Update);
    assert_eq!(chunk.replay_policy, ReplayPolicy::QueryOrReadOnly);

    let publish = STORE_ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_wasm_store_publish_chunk")
        .expect("Store publish-chunk lane");
    assert_eq!(publish.endpoint_kind, EndpointKind::Update);
    assert!(matches!(
        publish.replay_policy,
        ReplayPolicy::MonotonicTransition { command_kind }
            if command_kind.as_str() == "wasm_store.publish_chunk.v1"
    ));
}

#[test]
fn root_membership_update_has_read_only_replay_authority() {
    let membership = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::CANIC_ROOT_MEMBERSHIP)
        .expect("Root membership policy entry");
    assert_eq!(membership.endpoint_kind, EndpointKind::Update);
    assert_eq!(membership.replay_policy, ReplayPolicy::QueryOrReadOnly);
    assert_eq!(membership.cost_class, CostClass::None);
}

#[test]
fn native_authority_mutations_keep_their_operation_replay_contracts() {
    let admission = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::admission_control::mutate::METHOD)
        .unwrap();
    assert_eq!(admission.endpoint_kind, EndpointKind::Update);
    assert_eq!(
        admission.replay_policy,
        ReplayPolicy::ReplayProtected {
            command_kind: replay_command_kind("fleet_admission.mutate.v1"),
            requires_operation_id: true,
        }
    );
    for (method, kind) in [
        (
            crate::protocol::protected_state::snapshot_prepare::METHOD,
            "authority_restore.prepare_snapshot.v1",
        ),
        (
            crate::protocol::protected_state::snapshot_resume::METHOD,
            "authority_restore.resume_snapshot.v1",
        ),
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        assert_eq!(entry.endpoint_kind, EndpointKind::Update);
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::ResponseIdempotent {
                command_kind: replay_command_kind(kind)
            }
        );
    }
}

#[test]
fn nonroot_cycles_keeps_the_existing_request_cycles_replay_and_cost_contract() {
    let endpoint = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::cycles_capability::METHOD)
        .unwrap();
    let capability = ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.variant == "RequestCycles")
        .unwrap();
    assert_eq!(endpoint.endpoint_kind, EndpointKind::Update);
    assert_eq!(endpoint.replay_policy, capability.replay_policy);
    assert_eq!(endpoint.cost_class, capability.cost_class);
    assert_eq!(endpoint.quota_policy, capability.quota_policy);
    assert_eq!(
        endpoint.cycle_reserve_policy,
        capability.cycle_reserve_policy
    );
}

#[test]
fn store_catalog_methods_are_independent_uncosted_queries() {
    for method in [
        crate::protocol::store_catalog::fixture::METHOD,
        crate::protocol::store_catalog::grant::METHOD,
        crate::protocol::store_catalog::template::METHOD,
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        assert_eq!(entry.endpoint_kind, EndpointKind::Query);
        assert_eq!(entry.replay_policy, ReplayPolicy::QueryOrReadOnly);
        assert_eq!(entry.cost_class, CostClass::None);
        assert_eq!(entry.quota_policy, None);
        assert_eq!(entry.cycle_reserve_policy, None);
    }
}

#[test]
fn root_activation_retains_response_idempotent_identity_and_no_paid_effect() {
    for (method, kind) in [
        (
            crate::protocol::root_activation::prepare::METHOD,
            "fleet_activation.prepare.v1",
        ),
        (
            crate::protocol::root_activation::resume::METHOD,
            "fleet_activation.resume.v1",
        ),
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        assert_eq!(entry.endpoint_kind, EndpointKind::Update);
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::ResponseIdempotent {
                command_kind: replay_command_kind(kind)
            }
        );
        assert_eq!(entry.cost_class, CostClass::None);
    }
}

#[test]
fn funding_controls_keep_exact_replay_labels_and_paid_request_cost() {
    for (method, label, protected, cost) in [
        (
            crate::protocol::funding_control::root_accept::METHOD,
            "fleet_root_funding.accept.v1",
            true,
            CostClass::None,
        ),
        (
            crate::protocol::funding_control::root_prepare::METHOD,
            "fleet_funding_policy_rotation.prepare_root.v1",
            true,
            CostClass::None,
        ),
        (
            crate::protocol::funding_control::root_activate::METHOD,
            "fleet_funding_policy_rotation.activate_root.v1",
            true,
            CostClass::None,
        ),
        (
            crate::protocol::funding_control::request::METHOD,
            "fleet_root_funding.request.v1",
            true,
            CostClass::ValueTransfer,
        ),
        (
            crate::protocol::funding_control::begin::METHOD,
            "fleet_funding_policy_rotation.begin.v1",
            true,
            CostClass::None,
        ),
        (
            crate::protocol::funding_control::stage::METHOD,
            "fleet_funding_policy_rotation.stage_root.v1",
            true,
            CostClass::None,
        ),
        (
            crate::protocol::funding_control::apply::METHOD,
            "fleet_funding_policy_rotation.apply.v1",
            true,
            CostClass::None,
        ),
        (
            crate::protocol::funding_control::set_enabled::METHOD,
            "fleet_root_funding.set_enabled.v1",
            false,
            CostClass::None,
        ),
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        let policy = if protected {
            ReplayPolicy::ReplayProtected {
                command_kind: replay_command_kind(label),
                requires_operation_id: true,
            }
        } else {
            ReplayPolicy::ResponseIdempotent {
                command_kind: replay_command_kind(label),
            }
        };
        assert_eq!(entry.endpoint_kind, EndpointKind::Update);
        assert_eq!(entry.replay_policy, policy);
        assert_eq!(entry.cost_class, cost);
    }
}

#[test]
fn allocation_refill_and_acknowledgement_keep_replay_authority() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| {
            entry.endpoint == crate::protocol::coordinator_acknowledgements::acknowledge::METHOD
        })
        .unwrap();
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("fleet_registry.acknowledge_root.v1"),
        }
    );
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::cycle_refill::preview::METHOD)
        .unwrap();
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(entry.replay_policy, ReplayPolicy::QueryOrReadOnly);
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::cycle_refill::execute::METHOD)
        .unwrap();
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ReplayProtected {
            command_kind: replay_command_kind("icp.refill.v1"),
            requires_operation_id: true,
        }
    );
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::component_control::child::METHOD)
        .unwrap();
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ReplayProtected {
            command_kind: replay_command_kind("root.provision_child.v1"),
            requires_operation_id: true,
        }
    );
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::component_control::peer::METHOD)
        .unwrap();
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ReplayProtected {
            command_kind: replay_command_kind("root.provision_peer.v1"),
            requires_operation_id: true,
        }
    );
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::component_control::remove::METHOD)
        .unwrap();
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ReplayProtected {
            command_kind: replay_command_kind("root.remove_component.v1"),
            requires_operation_id: true,
        }
    );
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::component_control::subtree::METHOD)
        .unwrap();
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ReplayProtected {
            command_kind: replay_command_kind("root.remove_subtree.v1"),
            requires_operation_id: true,
        }
    );
}

#[test]
fn registry_deletion_and_handoff_keep_original_replay_kinds() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::root_deletion::complete::METHOD)
        .unwrap();
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("fleet_registry.complete_root_deletion.v1")
        }
    );
    assert_eq!(entry.cost_class, CostClass::None);
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::root_deletion::prepare::METHOD)
        .unwrap();
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("fleet_registry.begin_root_deletion_execution.v1")
        }
    );
    assert_eq!(entry.cost_class, CostClass::None);
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::pool_control::handoff::METHOD)
        .unwrap();
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::SnapshotConvergent {
            command_kind: replay_command_kind("canister_pool.handoff.v1")
        }
    );
    assert_eq!(entry.cost_class, CostClass::None);
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::registry_control::join::METHOD)
        .unwrap();
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("fleet_registry.root_join.v1")
        }
    );
    assert_eq!(entry.cost_class, CostClass::None);
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::registry_control::activate::METHOD)
        .unwrap();
    assert_eq!(entry.endpoint_kind, EndpointKind::Update);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("fleet_registry.activate.v1")
        }
    );
    assert_eq!(entry.cost_class, CostClass::None);
}

#[test]
fn root_store_controls_preserve_replay_labels() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::root_store_control::adopt::METHOD)
        .unwrap();
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ReplayProtected {
            command_kind: replay_command_kind("root.adopt_store.v1"),
            requires_operation_id: true,
        }
    );
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::root_store_control::bootstrap::METHOD)
        .unwrap();
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ReplayProtected {
            command_kind: replay_command_kind("root.bootstrap_store.v1"),
            requires_operation_id: true,
        }
    );
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::root_store_control::prepare::METHOD)
        .unwrap();
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::MonotonicTransition {
            command_kind: replay_command_kind("root.prepare_store_fixture.v1"),
        }
    );
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::root_store_control::publish::METHOD)
        .unwrap();
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::MonotonicTransition {
            command_kind: replay_command_kind("wasm_store.admin.v1"),
        }
    );
}

#[test]
fn registry_pool_and_component_controls_keep_their_replay_owners() {
    use crate::protocol::{component_control, pool_control, registry_control};
    for (method, label, policy, cost, quota, reserve) in [
        (
            registry_control::synchronize::METHOD,
            "fleet_registry.synchronize_root.v1",
            0,
            CostClass::None,
            None,
            None,
        ),
        (
            component_control::provision::METHOD,
            "root.provision_component.v1",
            0,
            CostClass::ManagementDeployment,
            Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
            Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
        ),
        (
            component_control::prepare::METHOD,
            "component_registry.prepare_root.v1",
            1,
            CostClass::None,
            None,
            None,
        ),
        (
            pool_control::maintain::METHOD,
            "canister_pool.maintain.v1",
            2,
            CostClass::ManagementDeployment,
            Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
            Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
        ),
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        let command_kind = replay_command_kind(label);
        let expected = match policy {
            0 => ReplayPolicy::ReplayProtected {
                command_kind,
                requires_operation_id: true,
            },
            1 => ReplayPolicy::ResponseIdempotent { command_kind },
            _ => ReplayPolicy::SnapshotConvergent { command_kind },
        };
        assert_eq!(entry.replay_policy, expected);
        assert_eq!(entry.cost_class, cost);
        assert_eq!(entry.quota_policy, quota);
        assert_eq!(entry.cycle_reserve_policy, reserve);
    }
}

#[test]
fn removal_and_retirement_preserve_original_operation_replay_labels() {
    use crate::protocol::retirement_control as wire;
    for (method, label) in [
        (wire::remove::METHOD, "coordinator.remove_root.v1"),
        (wire::root_remove::METHOD, "root.remove_root.v1"),
        (wire::retire::METHOD, "fleet.retirement.ledger.transfer.v1"),
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::ReplayProtected {
                command_kind: replay_command_kind(label),
                requires_operation_id: true
            }
        );
        if method == wire::remove::METHOD {
            assert_eq!(entry.cost_class, CostClass::None);
        } else {
            assert_eq!(entry.cost_class, CostClass::ValueTransfer);
            assert_eq!(
                entry.quota_policy,
                Some(crate::replay_policy::quota::VALUE_TRANSFER_QUOTA_V1)
            );
            assert_eq!(
                entry.cycle_reserve_policy,
                Some(crate::replay_policy::quota::VALUE_TRANSFER_RESERVE_V1)
            );
        }
    }
}

#[test]
fn native_batch_mutations_keep_original_replay_ownership_and_limits() {
    use crate::protocol::batch_control as wire;
    for (method, label, protected) in [
        (
            wire::prepare::METHOD,
            "coordinator.provision_components.v1",
            true,
        ),
        (
            wire::accept::METHOD,
            "component_provisioning.accept_root_batch.v1",
            false,
        ),
        (
            wire::synchronize::METHOD,
            "component_provisioning.synchronize_affected_directories.v1",
            false,
        ),
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        let command_kind = replay_command_kind(label);
        let expected = if protected {
            ReplayPolicy::ReplayProtected {
                command_kind,
                requires_operation_id: true,
            }
        } else {
            ReplayPolicy::ResponseIdempotent { command_kind }
        };
        assert_eq!(entry.replay_policy, expected);
        assert_eq!(entry.cost_class, CostClass::None);
        assert_eq!(entry.quota_policy, None);
        assert_eq!(entry.cycle_reserve_policy, None);
    }
    assert_eq!(wire::PLAN_LIMITS.bytes, crate::ops::component_provisioning_plan::MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CANONICAL_BYTES);
    const {
        assert!(wire::ROOT_LIMITS.bytes > crate::ops::component_provisioning_plan::MAX_FLEET_SUBNET_ROOT_PROVISIONING_BATCH_CANONICAL_BYTES);
    }
}

#[test]
fn import_methods_retain_original_paid_effect_policy() {
    use crate::protocol::{import_control as wire, pool_control};
    for method in [
        wire::reserve::METHOD,
        wire::advance::METHOD,
        wire::settle::METHOD,
        wire::release::METHOD,
        pool_control::import::METHOD,
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        let expected = if method == pool_control::import::METHOD {
            ReplayPolicy::SnapshotConvergent {
                command_kind: replay_command_kind("canister_pool.import.v1"),
            }
        } else {
            ReplayPolicy::ReplayProtected {
                command_kind: replay_command_kind("canister_pool.capacity_import.v1"),
                requires_operation_id: true,
            }
        };
        assert_eq!(entry.replay_policy, expected);
        assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
        assert_eq!(
            entry.quota_policy,
            Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1)
        );
        assert_eq!(
            entry.cycle_reserve_policy,
            Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1)
        );
    }
}
