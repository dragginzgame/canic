//! Module: component_topology::tests
//!
//! Responsibility: verify host finalization of immutable Fleet Subnet Root topology bindings.
//! Does not own: network discovery, root creation, release-set construction, or installation.
//! Boundary: exercises canonicalization and Fleet-scoped admission/placement invariants.

use canic_core::{
    bootstrap::{compiled::ConfigModel, parse_config_model},
    cdk::types::Cycles,
    ids::{
        AppId, CanonicalNetworkId, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding,
        FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootLimits, SubnetId,
    },
};

use super::*;

const CONFIG: &str = r#"
[app]
name = "toko"

[roles.root]
kind = "root"
package = "root"

[roles.database]
kind = "canister"
package = "database"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[component_specs.database]
component_role = "database"
maximum_instances = 2

[component_specs.users]
component_role = "user_hub"
maximum_instances = 3
"#;

const EMPTY_APPLICATION_CONFIG: &str = r#"
[app]
name = "reserve"

[roles.root]
kind = "root"
package = "root"
"#;

fn config() -> ConfigModel {
    parse_config_model(CONFIG).expect("valid topology config")
}

fn authority(fleet_byte: u8) -> FleetRegistryAuthority {
    FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([fleet_byte; 32]),
                },
                app: AppId::from("toko"),
            },
            coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[1; 29])),
            coordinator: Principal::from_slice(&[2; 29]),
        },
        epoch: 1,
    }
}

fn limits() -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        maximum_component_instances: 10,
        maximum_registry_bytes: 4_194_304,
        maximum_wasm_store_bytes: 40_000_000,
        maximum_group_placements: 16,
        canister_pool: canic_core::ids::FleetSubnetCanisterPoolConfig {
            minimum_size: 1,
            maximum_size: 10,
            canister_cycles: Cycles::new(5_000_000_000_000),
        },
        cycles_funding: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(1_000_000_000_000_000),
        },
    }
}

fn admission(component_spec: &str, maximum_root_instances: u32) -> RootComponentAdmissionInput {
    RootComponentAdmissionInput {
        component_spec: component_spec.parse().expect("Component Spec ID"),
        maximum_root_instances,
    }
}

#[test]
fn retained_pool_imports_must_fit_the_root_initialisation_maximum() {
    let input = RootPoolImportCapacityInput {
        import_count: 3,
        maximum_size: 2,
        root: "root-0".to_string(),
    };
    assert_eq!(
        validate_root_pool_import_capacity(&input),
        Err(RootPoolImportCapacityError {
            import_count: 3,
            maximum_size: 2,
            root: "root-0".to_string(),
        })
    );

    assert!(
        validate_root_pool_import_capacity(&RootPoolImportCapacityInput {
            import_count: 3,
            maximum_size: 3,
            root: "root-0".to_string(),
        })
        .is_ok()
    );
}

fn root(
    root_byte: u8,
    subnet_byte: u8,
    component_admissions: Vec<RootComponentAdmissionInput>,
) -> FleetSubnetRootTopologyInput {
    FleetSubnetRootTopologyInput {
        placement_subnet: SubnetId::from_principal(Principal::from_slice(&[subnet_byte; 29])),
        fleet_subnet_root: Principal::from_slice(&[root_byte; 29]),
        component_admissions,
        limits: limits(),
        funding: crate::test_support::fleet_subnet_root_funding_authority(),
    }
}

fn planned_root(
    subnet_byte: u8,
    component_admissions: Vec<RootComponentAdmissionInput>,
) -> PlannedFleetSubnetRootTopologyInput {
    PlannedFleetSubnetRootTopologyInput {
        placement_subnet: SubnetId::from_principal(Principal::from_slice(&[subnet_byte; 29])),
        component_admissions,
        limits: limits(),
    }
}

#[test]
fn pre_creation_planner_derives_complete_topology_without_canister_principals() {
    let plan = plan_initial_fleet_topology(
        &config(),
        vec![
            planned_root(7, vec![admission("users", 2), admission("database", 1)]),
            planned_root(5, vec![admission("users", 1), admission("database", 1)]),
        ],
    )
    .expect("valid pre-creation topology plan");

    assert_eq!(
        plan.fleet_subnet_roots
            .iter()
            .map(|root| root.placement_subnet)
            .collect::<Vec<_>>(),
        vec![
            SubnetId::from_principal(Principal::from_slice(&[5; 29])),
            SubnetId::from_principal(Principal::from_slice(&[7; 29])),
        ]
    );
    for root in &plan.fleet_subnet_roots {
        assert_eq!(
            root.component_topology_digest,
            plan.component_topology
                .project_for_admissions(&root.component_admissions)
                .expect("project root")
                .digest()
                .expect("root topology digest")
        );
    }

    std::assert_matches!(
        plan_initial_fleet_topology(
            &config(),
            vec![
                planned_root(5, vec![admission("database", 1)]),
                planned_root(5, vec![admission("users", 1)]),
            ],
        ),
        Err(FleetTopologyPlanError::DuplicatePlacementSubnet { .. })
    );
}

#[test]
fn admitted_component_demand_cannot_exceed_root_pool_target() {
    let config = config();
    let plan = plan_initial_fleet_topology(
        &config,
        vec![planned_root(
            5,
            vec![admission("database", 1), admission("users", 1)],
        )],
    )
    .expect("valid planned Root");
    let admitted = plan.fleet_subnet_roots[0].component_admissions.clone();
    let root = Principal::from_slice(&[4; 29]).to_text();

    let insufficient = RootPoolCapacityInput {
        component_admissions: admitted.clone(),
        pool_target_cycles: 4_999_999_999_999,
        root: root.clone(),
    };
    assert_eq!(
        validate_root_pool_capacity(&config, &[insufficient]),
        Err(RootPoolCapacityError::Insufficient {
            component_spec: "database".parse().expect("Component Spec"),
            pool_target_cycles: 4_999_999_999_999,
            required_cycles: 5_000_000_000_000,
            root: root.clone(),
        })
    );

    validate_root_pool_capacity(
        &config,
        &[RootPoolCapacityInput {
            component_admissions: admitted,
            pool_target_cycles: 5_000_000_000_000,
            root,
        }],
    )
    .expect("equal pool target admits the Component");
}

#[test]
fn empty_application_topology_reports_the_current_projection_blocker() {
    let config = parse_config_model(EMPTY_APPLICATION_CONFIG).expect("valid empty application");

    std::assert_matches!(
        plan_initial_fleet_topology(&config, vec![planned_root(5, Vec::new())]),
        Err(FleetTopologyPlanError::Topology(
            canic_core::bootstrap::compiled::ComponentTopologyError::EmptyRootAdmissions
        ))
    );
}

#[test]
fn planner_derives_hashes_and_root_digests_from_canonical_config() {
    let plan = plan_fleet_topology(
        &config(),
        authority(3),
        vec![
            root(4, 5, vec![admission("users", 1), admission("database", 1)]),
            root(6, 7, vec![admission("users", 2), admission("database", 1)]),
        ],
    )
    .expect("valid Fleet topology plan");

    assert_eq!(plan.fleet_subnet_roots.len(), 2);
    for root in &plan.fleet_subnet_roots {
        assert_eq!(
            root.component_admissions
                .iter()
                .map(|admission| admission.component_spec.as_str())
                .collect::<Vec<_>>(),
            vec!["database", "users"],
        );
        let projection = plan
            .component_topology
            .project_for_admissions(&root.component_admissions)
            .expect("root projection");
        assert_eq!(
            root.component_topology_digest,
            projection.digest().expect("root digest"),
        );
    }
}

#[test]
fn planner_rejects_duplicate_unknown_zero_and_excess_admissions() {
    let config = config();

    std::assert_matches!(
        plan_fleet_topology(
            &config,
            authority(3),
            vec![root(
                4,
                5,
                vec![admission("users", 1), admission("users", 1)],
            )],
        ),
        Err(FleetTopologyPlanError::DuplicateAdmission { .. })
    );

    std::assert_matches!(
        plan_fleet_topology(
            &config,
            authority(3),
            vec![root(4, 5, vec![admission("unknown", 1)])],
        ),
        Err(FleetTopologyPlanError::UnknownComponentSpec { .. })
    );

    std::assert_matches!(
        plan_fleet_topology(
            &config,
            authority(3),
            vec![root(
                4,
                5,
                vec![admission("database", 1), admission("users", 0)],
            )],
        ),
        Err(FleetTopologyPlanError::Topology(
            canic_core::bootstrap::compiled::ComponentTopologyError::ZeroRootAdmission { .. }
        ))
    );

    std::assert_matches!(
        plan_fleet_topology(
            &config,
            authority(3),
            vec![root(
                4,
                5,
                vec![admission("database", 1), admission("users", 4)],
            )],
        ),
        Err(FleetTopologyPlanError::Topology(
            canic_core::bootstrap::compiled::ComponentTopologyError::RootAdmissionExceedsFleetMaximum { .. }
        ))
    );
}

#[test]
fn one_fleet_rejects_duplicate_subnet_but_different_fleets_may_reuse_it() {
    let config = config();
    let first = root(4, 5, vec![admission("database", 1), admission("users", 1)]);
    let second_same_subnet = root(6, 5, vec![admission("database", 1), admission("users", 2)]);

    std::assert_matches!(
        plan_fleet_topology(
            &config,
            authority(3),
            vec![first, second_same_subnet],
        ),
        Err(FleetTopologyPlanError::Topology(
            canic_core::bootstrap::compiled::ComponentTopologyError::DuplicateFleetSubnetRootSubnet { .. }
        ))
    );

    for fleet_byte in [3, 8] {
        plan_fleet_topology(
            &config,
            authority(fleet_byte),
            vec![root(
                fleet_byte + 10,
                5,
                vec![admission("database", 2), admission("users", 3)],
            )],
        )
        .expect("different Fleet may independently use the same physical Subnet");
    }
}

#[test]
fn planner_binds_the_authority_to_the_configured_app() {
    let mut wrong = authority(3);
    wrong.binding.fleet.app = AppId::from("another_app");

    std::assert_matches!(
        plan_fleet_topology(
            &config(),
            wrong,
            vec![root(
                4,
                5,
                vec![admission("database", 2), admission("users", 3)],
            )],
        ),
        Err(FleetTopologyPlanError::AppMismatch { .. })
    );
}
