//! Module: replay_policy::tests::role_command
//!
//! Responsibility: verify exact Root and Coordinator command-variant replay ownership.
//! Does not own: endpoint dispatch or workflow implementation.
//! Boundary: source-declared closed command unions must match their policy manifests.

use super::*;

#[test]
fn managed_observability_does_not_acquire_mutation_replay_policy() {
    for method in [
        crate::protocol::observability_relay::child_funding::METHOD,
        crate::protocol::observability_relay::cycle_balance::METHOD,
        crate::protocol::observability_relay::cycle_history::METHOD,
        crate::protocol::observability_relay::cycle_topups::METHOD,
        crate::protocol::observability_relay::memory_allocations::METHOD,
        crate::protocol::observability_relay::metrics::METHOD,
        crate::protocol::root_observation::child_funding::METHOD,
        crate::protocol::root_observation::cycle_balance::METHOD,
        crate::protocol::root_observation::cycle_history::METHOD,
        crate::protocol::root_observation::cycle_topups::METHOD,
        crate::protocol::root_observation::memory_allocations::METHOD,
        crate::protocol::root_observation::metrics::METHOD,
    ] {
        let entry = crate::replay_policy::endpoint_manifest::ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .expect("replicated observation policy");
        assert_eq!(entry.endpoint_kind, EndpointKind::Update);
        assert_eq!(entry.replay_policy, ReplayPolicy::QueryOrReadOnly);
        assert_eq!(entry.cost_class, CostClass::None);
    }
}

#[test]
fn managed_publication_and_startup_retain_their_exact_replay_owners() {
    use crate::protocol::caller_authority as protocol;
    let entry = |name| {
        crate::replay_policy::endpoint_manifest::ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == name)
            .unwrap()
    };
    for method in [protocol::PREPARE, protocol::COMMIT, protocol::COMPLETE] {
        let publication = entry(method);
        assert_eq!(
            publication.replay_policy,
            ReplayPolicy::ReplayProtected {
                command_kind: replay_command_kind("caller_authority.publication.v1"),
                requires_operation_id: true,
            }
        );
        assert_eq!(publication.cost_class, CostClass::None);
    }
    assert_eq!(
        entry(protocol::RELEASE).replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("application_startup.release.v1"),
        }
    );
    assert_eq!(entry(protocol::RELEASE).cost_class, CostClass::None);
}

#[test]
fn root_capability_methods_keep_the_exact_existing_replay_contracts() {
    use crate::protocol::root_capability as wire;
    for (method, variant) in [
        (wire::acknowledge::METHOD, "AcknowledgePlacementReceipt"),
        (wire::allocate::METHOD, "AllocatePlacementChild"),
        (wire::create::METHOD, "ProvisionCanister"),
        (wire::recycle::METHOD, "RecycleCanister"),
        (wire::cycles::METHOD, "RequestCycles"),
    ] {
        let endpoint = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        let capability = command_entry(ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST, variant);
        assert_eq!(endpoint.replay_policy, capability.replay_policy);
        assert_eq!(endpoint.cost_class, capability.cost_class);
        assert_eq!(endpoint.quota_policy, capability.quota_policy);
        assert_eq!(
            endpoint.cycle_reserve_policy,
            capability.cycle_reserve_policy
        );
    }
}

#[test]
fn managed_auth_effects_own_exact_replay_contracts() {
    let install = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::issuer_proof::INSTALL)
        .unwrap();
    assert!(matches!(
        install.replay_policy,
        ReplayPolicy::IntentionallyNonIdempotent { .. }
    ));
}

#[test]
fn managed_admission_transitions_are_operation_replay_protected() {
    for method in [
        crate::protocol::admission_control::target_prepare::METHOD,
        crate::protocol::admission_control::target_activate::METHOD,
        crate::protocol::admission_control::target_open::METHOD,
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        assert!(matches!(
            entry.replay_policy,
            ReplayPolicy::ReplayProtected {
                requires_operation_id: true,
                ..
            }
        ));
        assert_eq!(entry.cost_class, CostClass::None);
    }
}

#[test]
fn root_admission_transitions_are_operation_replay_protected() {
    for method in [
        crate::protocol::admission_control::root_prepare::METHOD,
        crate::protocol::admission_control::root_activate::METHOD,
        crate::protocol::admission_control::root_open::METHOD,
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        assert!(matches!(
            entry.replay_policy,
            ReplayPolicy::ReplayProtected {
                requires_operation_id: true,
                ..
            }
        ));
        assert_eq!(entry.cost_class, CostClass::None);
    }
}

#[test]
fn store_effect_variants_preserve_cost_and_convergence() {
    let reclaim = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == crate::protocol::store_control::reclaim::METHOD)
        .unwrap();
    assert!(matches!(
        reclaim.replay_policy,
        ReplayPolicy::SnapshotConvergent { .. }
    ));
    assert_eq!(reclaim.cost_class, CostClass::ValueTransfer);
    assert_eq!(reclaim.quota_policy, Some(VALUE_TRANSFER_QUOTA_V1));
    assert_eq!(
        reclaim.cycle_reserve_policy,
        Some(VALUE_TRANSFER_RESERVE_V1)
    );

    for method in [
        crate::protocol::cascade::state::METHOD,
        crate::protocol::cascade::topology::METHOD,
    ] {
        assert!(matches!(
            ENDPOINT_REPLAY_POLICY_MANIFEST
                .iter()
                .find(|entry| entry.endpoint == method)
                .unwrap()
                .replay_policy,
            ReplayPolicy::SnapshotConvergent { .. }
        ));
    }
}

fn command_entry(
    manifest: &'static [CommandReplayPolicy],
    variant: &str,
) -> &'static CommandReplayPolicy {
    manifest
        .iter()
        .find(|entry| entry.variant == variant)
        .unwrap_or_else(|| panic!("missing command replay policy for {variant}"))
}

#[test]
fn pool_retry_methods_preserve_convergence_and_spending_policy() {
    use crate::replay_policy::quota::{DEPLOYMENT_QUOTA_V1, DEPLOYMENT_RESERVE_V1};
    for (method, cost, quota, reserve) in [
        (
            crate::protocol::pool_retry::refill::METHOD,
            CostClass::ValueTransfer,
            VALUE_TRANSFER_QUOTA_V1,
            VALUE_TRANSFER_RESERVE_V1,
        ),
        (
            crate::protocol::pool_retry::reset::METHOD,
            CostClass::ManagementDeployment,
            DEPLOYMENT_QUOTA_V1,
            DEPLOYMENT_RESERVE_V1,
        ),
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        assert!(matches!(
            entry.replay_policy,
            ReplayPolicy::SnapshotConvergent { .. }
        ));
        assert_eq!(entry.cost_class, cost);
        assert_eq!(entry.quota_policy, Some(quota));
        assert_eq!(entry.cycle_reserve_policy, Some(reserve));
    }
}

#[test]
fn fleet_state_methods_retain_response_idempotence_without_paid_effect_authority() {
    for method in [
        crate::protocol::store_activation::activate::METHOD,
        crate::protocol::component_initialization::METHOD,
        crate::protocol::fleet_state_control::funding::METHOD,
        crate::protocol::fleet_state_control::status::METHOD,
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        assert!(matches!(
            entry.replay_policy,
            ReplayPolicy::ResponseIdempotent { .. }
        ));
        assert_eq!(entry.cost_class, CostClass::None);
        assert_eq!(entry.quota_policy, None);
        assert_eq!(entry.cycle_reserve_policy, None);
    }
}

#[test]
fn store_controls_keep_existing_monotonic_authority_without_paid_effects() {
    for method in [
        crate::protocol::store_control::grant::METHOD,
        crate::protocol::store_activation::prepare::METHOD,
        crate::protocol::store_publication::prepare::METHOD,
        crate::protocol::store_publication::stage::METHOD,
        crate::protocol::store_publication::fixture_prepare::METHOD,
        crate::protocol::store_gc::METHOD,
    ] {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == method)
            .unwrap();
        assert!(matches!(
            entry.replay_policy,
            ReplayPolicy::MonotonicTransition { .. }
        ));
        assert_eq!(entry.cost_class, CostClass::None);
        assert_eq!(entry.quota_policy, None);
        assert_eq!(entry.cycle_reserve_policy, None);
    }
}
