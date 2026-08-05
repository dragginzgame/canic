//! Focused proofs for canonical Fleet-service target compilation.

use super::*;
use crate::config::{Config, ConfigError};

const CONFIG_PREFIX: &str = r#"
[app]
name = "fleet_services"

[roles.root]
kind = "root"
package = "root"

[roles.a]
kind = "canister"
package = "a"

[roles.b]
kind = "canister"
package = "b"

[component_specs.a]
component_role = "a"
maximum_instances = 8

[component_specs.b]
component_role = "b"
maximum_instances = 8
"#;

fn parse(source: &str) -> Result<(ConfigModel, FleetServiceTopology), ConfigError> {
    let config = Config::parse_toml(&format!("{CONFIG_PREFIX}\n{source}"))?;
    let topology = config.compile_fleet_service_topology()?;
    Ok((config, topology))
}

const VALID_SERVICES: &str = r#"
[component_groups.database.components.database]
component_spec = "a"
service = "database"

[component_groups.api.components.api]
component_spec = "b"
service = "api"

[component_group_deployments.authoritative]
component_group = "database"
service_purpose = "authority"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[component_group_deployments.replicas]
component_group = "database"
service_purpose = "replica"
initial_placements = 1
maximum_placements = 3
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[component_group_deployments.api]
component_group = "api"
service_purpose = "pool_member"
initial_placements = 2
maximum_placements = 4
placement.maximum_per_root = 2
placement.minimum_distinct_roots = 2

[services.fleet.targets.database]
role = "a"
component_spec = "a"
mode = "authority_replica"
authority_deployment = "authoritative"
authority_member = ["database"]
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 1

[services.fleet.targets.api]
role = "b"
component_spec = "b"
mode = "active_pool"
placement.maximum_members_per_root = 2
placement.minimum_distinct_roots = 2
"#;

#[test]
fn compiles_canonical_mode_compatible_targets_and_revalidates_candid() {
    let (config, topology) = parse(VALID_SERVICES).expect("valid Fleet services");

    assert_eq!(
        topology
            .targets
            .iter()
            .map(|target| target.service.as_str())
            .collect::<Vec<_>>(),
        vec!["api", "database"]
    );
    assert!(matches!(
        topology.targets[0].mode,
        FleetServiceTargetMode::ActivePool
    ));
    assert!(matches!(
        topology.targets[1].mode,
        FleetServiceTargetMode::AuthorityReplica { .. }
    ));

    let encoded = candid::encode_one(&topology).expect("encode Fleet-service topology");
    let decoded: FleetServiceTopology =
        candid::decode_one(&encoded).expect("decode Fleet-service topology");
    assert_eq!(decoded, topology);
    decoded
        .validate(
            &config,
            &config
                .compile_component_group_deployment_topology()
                .expect("deployment topology"),
            &config
                .compile_component_topology()
                .expect("Component topology"),
        )
        .expect("decoded topology remains exact");

    let mut wrong_order = topology.clone();
    wrong_order.targets.swap(0, 1);
    assert!(matches!(
        wrong_order.validate(
            &config,
            &config
                .compile_component_group_deployment_topology()
                .expect("deployment topology"),
            &config
                .compile_component_topology()
                .expect("Component topology"),
        ),
        Err(FleetServiceTopologyError::NonCanonicalTargetOrder { .. })
    ));

    let mut changed_policy = topology;
    changed_policy.targets[0].placement.maximum_members_per_root = 1;
    assert!(matches!(
        changed_policy.validate(
            &config,
            &config
                .compile_component_group_deployment_topology()
                .expect("deployment topology"),
            &config
                .compile_component_topology()
                .expect("Component topology"),
        ),
        Err(FleetServiceTopologyError::TargetProjectionMismatch)
    ));
}

#[test]
fn distinct_service_ids_may_share_one_role_and_component_spec() {
    parse(
        r#"
[component_groups.pools.components.first]
component_spec = "a"
service = "first"

[component_groups.pools.components.second]
component_spec = "a"
service = "second"

[component_group_deployments.pools]
component_group = "pools"
service_purpose = "pool_member"
initial_placements = 1
maximum_placements = 2
placement.maximum_per_root = 2
placement.minimum_distinct_roots = 1

[services.fleet.targets.first]
role = "a"
component_spec = "a"
mode = "active_pool"
placement.maximum_members_per_root = 2
placement.minimum_distinct_roots = 1

[services.fleet.targets.second]
role = "a"
component_spec = "a"
mode = "active_pool"
placement.maximum_members_per_root = 2
placement.minimum_distinct_roots = 1
"#,
    )
    .expect("distinct logical services may share one exact role and Spec");
}

#[test]
fn target_count_rejects_the_first_value_above_the_structural_bound() {
    let mut config = Config::parse_toml(CONFIG_PREFIX).expect("base config");
    for index in 0..=MAX_FLEET_SERVICE_TARGETS {
        config.services.fleet.targets.insert(
            format!("s{index}")
                .parse()
                .expect("bounded Fleet service ID"),
            FleetServiceTargetConfig::ActivePool {
                role: CanisterRole::from("a"),
                component_spec: "a".parse().expect("Component Spec ID"),
                placement: FleetServicePlacementPolicyConfig {
                    maximum_members_per_root: 1,
                    minimum_distinct_roots: 1,
                },
            },
        );
    }

    assert!(matches!(
        config.compile_fleet_service_topology(),
        Err(FleetServiceTopologyError::TargetBoundExceeded {
            actual,
            maximum: MAX_FLEET_SERVICE_TARGETS,
        }) if actual == MAX_FLEET_SERVICE_TARGETS + 1
    ));
}

#[test]
fn orphan_occurrences_and_targets_reject() {
    let occurrence = parse(
        r#"
[component_groups.pool.components.a]
component_spec = "a"
service = "pool"
[component_group_deployments.pool]
component_group = "pool"
service_purpose = "pool_member"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#,
    )
    .expect_err("service occurrence without target must reject");
    assert!(matches!(
        occurrence,
        ConfigError::FleetServiceTopology(
            FleetServiceTopologyError::OrphanServiceOccurrence { .. }
        )
    ));

    let target = parse(
        r#"
[services.fleet.targets.pool]
role = "a"
component_spec = "a"
mode = "active_pool"
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 1
"#,
    )
    .expect_err("target without service occurrence must reject");
    assert!(matches!(
        target,
        ConfigError::FleetServiceTopology(FleetServiceTopologyError::OrphanServiceTarget { .. })
    ));
}

#[test]
fn target_role_and_occurrence_spec_must_match_exactly() {
    let wrong_role = parse(&VALID_SERVICES.replace("role = \"a\"", "role = \"b\""))
        .expect_err("target role must match its Component Spec");
    assert!(matches!(
        wrong_role,
        ConfigError::FleetServiceTopology(FleetServiceTopologyError::TargetRoleMismatch { .. })
    ));

    let wrong_spec = parse(&VALID_SERVICES.replace(
        "[services.fleet.targets.database]\nrole = \"a\"\ncomponent_spec = \"a\"",
        "[services.fleet.targets.database]\nrole = \"b\"\ncomponent_spec = \"b\"",
    ))
    .expect_err("every occurrence must use the target's exact Component Spec");
    assert!(matches!(
        wrong_spec,
        ConfigError::FleetServiceTopology(
            FleetServiceTopologyError::OccurrenceComponentSpecMismatch { .. }
        )
    ));
}

#[test]
fn target_modes_reject_incompatible_member_purposes() {
    let authority_as_pool = parse(
        &VALID_SERVICES.replace(
            "mode = \"authority_replica\"\nauthority_deployment = \"authoritative\"\nauthority_member = [\"database\"]",
            "mode = \"active_pool\"",
        ),
    )
    .expect_err("ActivePool must reject Authority and Replica occurrences");
    assert!(matches!(
        authority_as_pool,
        ConfigError::FleetServiceTopology(
            FleetServiceTopologyError::ActivePoolContainsNonPoolMember { .. }
        )
    ));

    let pool_as_authority = parse(
        &VALID_SERVICES.replace(
            "mode = \"active_pool\"\nplacement.maximum_members_per_root = 2",
            "mode = \"authority_replica\"\nauthority_deployment = \"api\"\nauthority_member = [\"api\"]\nplacement.maximum_members_per_root = 2",
        ),
    )
    .expect_err("AuthorityReplica must reject PoolMember occurrences");
    assert!(matches!(
        pool_as_authority,
        ConfigError::FleetServiceTopology(
            FleetServiceTopologyError::AuthorityReplicaContainsPoolMember { .. }
        )
    ));
}

#[test]
fn authority_replica_requires_one_exact_singleton_authority() {
    let missing = parse(&VALID_SERVICES.replace(
        "service_purpose = \"authority\"",
        "service_purpose = \"replica\"",
    ))
    .expect_err("AuthorityReplica must contain one Authority");
    assert!(matches!(
        missing,
        ConfigError::FleetServiceTopology(
            FleetServiceTopologyError::MissingServiceAuthority { .. }
        )
    ));

    let wrong_selector = parse(&VALID_SERVICES.replace(
        "authority_deployment = \"authoritative\"",
        "authority_deployment = \"replicas\"",
    ))
    .expect_err("authority selector must bind the exact Authority occurrence");
    assert!(matches!(
        wrong_selector,
        ConfigError::FleetServiceTopology(
            FleetServiceTopologyError::AuthoritySelectorMismatch { .. }
        )
    ));

    let scalable_authority = parse(&VALID_SERVICES.replace(
        "[component_group_deployments.authoritative]\ncomponent_group = \"database\"\nservice_purpose = \"authority\"\ninitial_placements = 1\nmaximum_placements = 1",
        "[component_group_deployments.authoritative]\ncomponent_group = \"database\"\nservice_purpose = \"authority\"\ninitial_placements = 1\nmaximum_placements = 2",
    ))
    .expect_err("Authority deployment must start and remain at one placement");
    assert!(matches!(
        scalable_authority,
        ConfigError::FleetServiceTopology(
            FleetServiceTopologyError::AuthorityDeploymentPlacementCountInvalid { .. }
        )
    ));

    let duplicate = parse(
        r#"
[component_groups.database.components.left]
component_spec = "a"
service = "database"
[component_groups.database.components.right]
component_spec = "a"
service = "database"
[component_group_deployments.authoritative]
component_group = "database"
service_purpose = "authority"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
[services.fleet.targets.database]
role = "a"
component_spec = "a"
mode = "authority_replica"
authority_deployment = "authoritative"
authority_member = ["left"]
placement.maximum_members_per_root = 2
placement.minimum_distinct_roots = 1
"#,
    )
    .expect_err("two Authority occurrences for one service must reject");
    assert!(matches!(
        duplicate,
        ConfigError::FleetServiceTopology(FleetServiceTopologyError::DuplicateServiceAuthority {
            actual: 2,
            ..
        })
    ));
}

#[test]
fn active_pool_requires_an_initial_member() {
    let error = parse(
        r#"
[component_groups.pool.components.a]
component_spec = "a"
service = "pool"
[component_group_deployments.pool]
component_group = "pool"
service_purpose = "pool_member"
initial_placements = 0
maximum_placements = 2
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
[services.fleet.targets.pool]
role = "a"
component_spec = "a"
mode = "active_pool"
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 1
"#,
    )
    .expect_err("ActivePool must initially materialize at least one member");
    assert!(matches!(
        error,
        ConfigError::FleetServiceTopology(
            FleetServiceTopologyError::ActivePoolHasNoInitialMember { .. }
        )
    ));
}

#[test]
fn service_placement_bounds_are_positive_and_within_maximum_members() {
    enum ExpectedError {
        ExcessDensity,
        ExcessSpread,
        ZeroDensity,
        ZeroSpread,
    }

    for (source, expected) in [
        (
            VALID_SERVICES.replace(
                "placement.maximum_members_per_root = 2",
                "placement.maximum_members_per_root = 0",
            ),
            ExpectedError::ZeroDensity,
        ),
        (
            VALID_SERVICES.replace(
                "placement.maximum_members_per_root = 2",
                "placement.maximum_members_per_root = 5",
            ),
            ExpectedError::ExcessDensity,
        ),
        (
            VALID_SERVICES.replace(
                "mode = \"active_pool\"\nplacement.maximum_members_per_root = 2\nplacement.minimum_distinct_roots = 2",
                "mode = \"active_pool\"\nplacement.maximum_members_per_root = 2\nplacement.minimum_distinct_roots = 0",
            ),
            ExpectedError::ZeroSpread,
        ),
        (
            VALID_SERVICES.replace(
                "mode = \"active_pool\"\nplacement.maximum_members_per_root = 2\nplacement.minimum_distinct_roots = 2",
                "mode = \"active_pool\"\nplacement.maximum_members_per_root = 2\nplacement.minimum_distinct_roots = 5",
            ),
            ExpectedError::ExcessSpread,
        ),
    ] {
        let error = parse(&source).expect_err("invalid service placement must reject");
        assert!(match expected {
            ExpectedError::ZeroDensity => matches!(
                error,
                ConfigError::FleetServiceTopology(
                    FleetServiceTopologyError::ZeroMaximumMembersPerRoot { .. }
                )
            ),
            ExpectedError::ExcessDensity => matches!(
                error,
                ConfigError::FleetServiceTopology(
                    FleetServiceTopologyError::MaximumMembersPerRootExceedsMaximum { .. }
                )
            ),
            ExpectedError::ZeroSpread => matches!(
                error,
                ConfigError::FleetServiceTopology(
                    FleetServiceTopologyError::ZeroMinimumDistinctRoots { .. }
                )
            ),
            ExpectedError::ExcessSpread => matches!(
                error,
                ConfigError::FleetServiceTopology(
                    FleetServiceTopologyError::MinimumDistinctRootsExceedsMaximum { .. }
                )
            ),
        });
    }
}

#[test]
fn service_placement_policy_must_fit_indivisible_group_placements() {
    let source = r#"
[component_groups.pool.components.left]
component_spec = "a"
service = "pool"
[component_groups.pool.components.right]
component_spec = "a"
service = "pool"
[component_group_deployments.pool]
component_group = "pool"
service_purpose = "pool_member"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
[services.fleet.targets.pool]
role = "a"
component_spec = "a"
mode = "active_pool"
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 1
"#;
    let narrow_root = parse(source)
        .expect_err("one group placement's same-service members must fit the service per-root cap");
    assert!(matches!(
        narrow_root,
        ConfigError::FleetServiceTopology(
            FleetServiceTopologyError::MaximumMembersPerRootBelowPlacementWidth {
                required_members_per_root: 2,
                ..
            }
        )
    ));

    let feasible_density = source.replace(
        "placement.maximum_members_per_root = 1",
        "placement.maximum_members_per_root = 2",
    );
    let impossible_spread = parse(&feasible_density.replace(
        "mode = \"active_pool\"\nplacement.maximum_members_per_root = 2\nplacement.minimum_distinct_roots = 1",
        "mode = \"active_pool\"\nplacement.maximum_members_per_root = 2\nplacement.minimum_distinct_roots = 2",
    ))
    .expect_err("one indivisible placement cannot occupy two distinct roots");
    assert!(matches!(
        impossible_spread,
        ConfigError::FleetServiceTopology(
            FleetServiceTopologyError::MinimumDistinctRootsExceedsMaximumPlacements {
                maximum_placements: 1,
                ..
            }
        )
    ));
}

#[test]
fn strict_source_rejects_mode_specific_extras_and_old_scalar_targets() {
    let conditional_extra = Config::parse_toml(&format!(
        "{CONFIG_PREFIX}\n{}",
        VALID_SERVICES.replace(
            "mode = \"active_pool\"\nplacement.maximum_members_per_root = 2",
            "mode = \"active_pool\"\nauthority_deployment = \"api\"\nplacement.maximum_members_per_root = 2",
        )
    ))
    .expect_err("ActivePool must reject Authority-only selector fields");
    assert!(matches!(
        conditional_extra,
        ConfigError::CannotParseToml { .. }
    ));

    let scalar = Config::parse_toml(&format!(
        "{CONFIG_PREFIX}\n[services.fleet]\ntargets = {{ pool = \"a\" }}\n"
    ))
    .expect_err("old scalar target shape must reject");
    assert!(matches!(scalar, ConfigError::CannotParseToml { .. }));
}
