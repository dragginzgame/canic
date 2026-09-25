use super::*;
use canic_control_plane::dto::fleet_coordinator::{
    CoordinatorFundingWindowStatusResponse, CoordinatorRootFundingStatusResponse,
};
use canic_core::{
    dto::{fleet_funding::FleetRootFundingRequest, fleet_registry::FleetSubnetRootStatus},
    ids::{CyclesFundingBudget, FleetFundingProfile},
};

#[test]
fn child_usage_requires_exact_participants_and_preserves_uncertain_reservations() {
    let parent = Principal::from_slice(&[1]);
    let child = Principal::from_slice(&[2]);
    let mut value = canic_core::dto::observability::ChildFundingUsage {
        parent,
        child,
        observed_at_ns: 10_000_000_000,
        accounted_cycles: 130.into(),
        last_accounted_at_secs: 9,
        pending_operations: 1,
        reserved_cycles: None,
    };
    let observed = project_child(value.clone(), parent, child).unwrap();
    assert_eq!(observed.accounted_cycles, 130);
    assert_eq!(observed.reserved_cycles, None);
    assert_eq!(observed.pending_operations, 1);
    assert_eq!(
        project_child(value.clone(), child, parent),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    value.pending_operations = 0;
    assert_eq!(
        project_child(value.clone(), parent, child),
        Err(StartupUsageUnavailable::InvalidAccounting)
    );
    value.reserved_cycles = Some(0.into());
    assert!(project_child(value.clone(), parent, child).is_ok());
    value.last_accounted_at_secs = 11;
    assert_eq!(
        project_child(value, parent, child),
        Err(StartupUsageUnavailable::InvalidAccounting)
    );
}

fn policy() -> FleetCoordinatorRootFundingPolicy {
    FleetCoordinatorRootFundingPolicy {
        funding_profile: FleetFundingProfile::SingleSubnet,
        minimum_reserve_cycles: 50.into(),
        budget: CyclesFundingBudget {
            window_secs: 60,
            maximum_cycles: 100.into(),
        },
        maximum_automatic_grants: 4,
        maximum_automatic_cycles: 200.into(),
    }
}

fn status() -> CoordinatorFundingStatusResponse {
    CoordinatorFundingStatusResponse {
        coordinator: Principal::from_slice(&[1]),
        current_cycles: 1_000.into(),
        policy_generation: 1,
        funding_enabled: true,
        funding_profile: Some(FleetFundingProfile::SingleSubnet),
        policy: Some(policy()),
        fleet_window: Some(CoordinatorFundingWindowStatusResponse {
            window_start_secs: 120,
            spent_cycles: 30.into(),
            reserved_cycles: 25.into(),
        }),
        historical_automatic_grants: 0,
        historical_automatic_cycles: 0.into(),
        automatic_grants: 2,
        automatic_cycles: 60.into(),
        rotation_checkpoint_count: 0,
        rotation_checkpoint_root_count: 0,
        rotation_checkpoint_root_capacity_remaining: 0,
        rotation: None,
        roots: vec![],
    }
}

#[test]
fn observed_usage_preserves_reservations_and_successful_lifetime_usage() {
    let mut value = status();
    value.funding_enabled = false;
    let result = project(
        value,
        Principal::from_slice(&[1]),
        &BTreeSet::new(),
        &policy(),
    )
    .unwrap();
    assert_eq!(
        (
            result.spent_cycles,
            result.reserved_cycles,
            result.window_remaining_cycles
        ),
        (30, 25, 45)
    );
    assert_eq!((result.automatic_grants, result.automatic_cycles), (2, 60));
    assert!(!result.funding_enabled);
    assert_eq!(result.pending_roots, 0);
}

#[test]
fn unavailable_usage_is_never_an_unused_budget() {
    let coordinator = Principal::from_slice(&[1]);
    let project_value = |value| project(value, coordinator, &BTreeSet::new(), &policy());
    let mut wrong = status();
    wrong.coordinator = Principal::from_slice(&[2]);
    assert_eq!(
        project_value(wrong),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    assert_eq!(
        project(
            status(),
            coordinator,
            &BTreeSet::from([Principal::from_slice(&[2])]),
            &policy()
        ),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    let mut changed = status();
    changed.policy.as_mut().unwrap().maximum_automatic_grants += 1;
    assert_eq!(
        project_value(changed),
        Err(StartupUsageUnavailable::PolicyTransition)
    );
    let mut rotating = status();
    rotating.rotation = Some(canic_control_plane::dto::fleet_coordinator::FleetFundingPolicyRotationStatusResponse {
        operation_id: [0; 32],
        plan_digest: [0; 32],
        predecessor_generation: 1,
        successor_generation: 2,
        phase: canic_control_plane::dto::fleet_coordinator::FleetFundingPolicyRotationStatusPhase::Staging {
            staged_root_count: 0,
            expected_root_count: 1,
        },
    });
    assert_eq!(
        project_value(rotating),
        Err(StartupUsageUnavailable::PolicyTransition)
    );
    let mut missing = status();
    missing.fleet_window = None;
    assert_eq!(
        project_value(missing),
        Err(StartupUsageUnavailable::InvalidAccounting)
    );
    let mut overflow = status();
    overflow.fleet_window.as_mut().unwrap().spent_cycles = u128::MAX.into();
    assert_eq!(
        project_value(overflow),
        Err(StartupUsageUnavailable::InvalidAccounting)
    );
    let mut excess = status();
    excess.automatic_grants = policy().maximum_automatic_grants + 1;
    assert_eq!(
        project_value(excess),
        Err(StartupUsageUnavailable::InvalidAccounting)
    );
}

#[test]
fn pending_root_operations_are_not_reported_as_successful_grants() {
    let root = Principal::from_slice(&[2]);
    let mut value = status();
    let root_policy = canic_core::ids::FleetSubnetRootFundingPolicy {
        funding_profile: FleetFundingProfile::SingleSubnet,
        request_threshold: 10.into(),
        target_balance: 30.into(),
        cooldown_secs: 60,
        budget: policy().budget,
        maximum_automatic_grants: 4,
        maximum_automatic_cycles: 200.into(),
    };
    let request = FleetRootFundingRequest {
        operation_id: [0; 32],
        operation_sequence: 1,
        expected_registry: canic_core::dto::fleet_registry::FleetRegistryVersion {
            authority: canic_core::ids::FleetRegistryAuthority {
                binding: canic_core::ids::FleetCoordinatorBinding {
                    fleet: canic_core::ids::FleetBinding {
                        fleet: canic_core::ids::FleetKey {
                            canonical_network_id: "11".repeat(32).parse().unwrap(),
                            fleet_id: "22".repeat(32).parse().unwrap(),
                        },
                        app: "test".into(),
                    },
                    coordinator_subnet: Principal::from_slice(&[3]).into(),
                    coordinator: value.coordinator,
                    recovery_controllers: Vec::new(),
                },
                epoch: 1,
            },
            revision: 1,
            content_hash: [0; 32],
        },
        observed_balance: 10.into(),
        requested_cycles: 20.into(),
        policy_hash: [0; 32],
    };
    value.roots.push(CoordinatorRootFundingStatusResponse {
        fleet_subnet_root: root,
        lifecycle_status: FleetSubnetRootStatus::Active,
        policy_hash: [0; 32],
        policy: root_policy,
        window: CoordinatorFundingWindowStatusResponse {
            window_start_secs: 120,
            spent_cycles: 0.into(),
            reserved_cycles: 20.into(),
        },
        historical_automatic_grants: 0,
        historical_automatic_cycles: 0.into(),
        automatic_grants: 1,
        automatic_cycles: 10.into(),
        last_successful_grant_at_ns: None,
        current_operation: Some(request),
        last_result: None,
    });
    let coordinator = value.coordinator;
    let mut duplicate = value.clone();
    duplicate.roots.push(duplicate.roots[0].clone());
    assert_eq!(
        project(duplicate, coordinator, &BTreeSet::from([root]), &policy()),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    let result = project(value, coordinator, &BTreeSet::from([root]), &policy()).unwrap();
    assert_eq!((result.pending_roots, result.automatic_grants), (1, 2));
}

#[cfg(unix)]
#[test]
fn protected_query_uses_canonical_candid_and_does_not_cache_usage() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = crate::test_support::temp_dir("startup-funding-observation");
    std::fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("icp");
    let response_file = directory.join("response.json");
    let script = crate::test_support::tool_script(&format!(
        "#!/bin/sh\ncase \"$*\" in\n  --version) printf 'icp @ICP_VERSION@\\n' ;;\n  *canic_observability*--query*) cat '{}' ;;\n  *) exit 1 ;;\nesac\n",
        response_file.display()
    ));
    std::fs::write(&executable, script).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let write = |value| {
        let bytes = candid::encode_one(Ok::<_, canic_core::dto::error::Error>(
            CoordinatorObservabilityResponse::Funding(value),
        ))
        .unwrap();
        std::fs::write(
            &response_file,
            serde_json::json!({"response_bytes": canic_core::cdk::utils::hash::hex_bytes(&bytes)})
                .to_string(),
        )
        .unwrap();
    };
    let icp = IcpCli::new(executable.to_str().unwrap(), None);
    let candid = directory.join("coordinator.did");
    std::fs::write(
        &candid,
        include_str!("../../../../../../canic/candid/fleet_coordinator.did"),
    )
    .unwrap();
    let observe_now = || {
        observe(
            &icp,
            &candid,
            Principal::from_slice(&[1]),
            &BTreeSet::new(),
            &policy(),
        )
    };
    write(status());
    let StartupCoordinatorUsage::Observed(first) = observe_now() else {
        panic!("protected current usage")
    };
    assert_eq!(first.window_remaining_cycles, 45);
    let mut next = status();
    next.fleet_window.as_mut().unwrap().reserved_cycles = 50.into();
    write(next);
    let StartupCoordinatorUsage::Observed(next) = observe_now() else {
        panic!("fresh current usage")
    };
    assert_eq!(next.window_remaining_cycles, 20);
    std::fs::write(response_file, "invalid response").unwrap();
    assert_eq!(
        observe_now(),
        StartupCoordinatorUsage::Unavailable(StartupUsageUnavailable::ObservationFailed)
    );
    std::fs::remove_dir_all(directory).unwrap();
}
