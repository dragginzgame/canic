//! Focused proofs for semantic Component deployment configuration identity.

use super::*;
use crate::config::{Config, ConfigError};

const BASELINE: &str = r#"
[app]
name = "configuration_digest"

[roles.root]
kind = "root"
package = "root"

[roles.database]
kind = "canister"
package = "database"

[roles.child]
kind = "canister"
package = "child"

[roles.api]
kind = "canister"
package = "api"

[component_specs.database]
component_role = "database"
maximum_instances = 4

[component_specs.database.children.child]
kind = "instance"

[component_specs.database.spawn_grants.database.child]
maximum_instances_per_parent = 10

[component_specs.api]
component_role = "api"
maximum_instances = 2

[component_groups.databases.components.database]
component_spec = "database"
service = "database"
labels = { tier = "storage" }

[component_groups.apis.components.api]
component_spec = "api"
service = "api"

[component_group_deployments.authoritative]
component_group = "databases"
service_purpose = "authority"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[component_group_deployments.replicas]
component_group = "databases"
service_purpose = "replica"
labels = { zone = "west" }
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[[component_group_deployments.replicas.member_limits]]
member = ["database"]
spawn_grants = [
  { parent_role = "database", child_role = "child", maximum_instances_per_parent = 5 },
]

[component_group_deployments.pool]
component_group = "apis"
service_purpose = "pool_member"
initial_placements = 1
maximum_placements = 2
placement.maximum_per_root = 2
placement.minimum_distinct_roots = 1

[services.fleet.targets.database]
role = "database"
component_spec = "database"
mode = "authority_replica"
authority_deployment = "authoritative"
authority_member = ["database"]
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 1

[services.fleet.targets.api]
role = "api"
component_spec = "api"
mode = "active_pool"
placement.maximum_members_per_root = 2
placement.minimum_distinct_roots = 1
"#;

fn digest(source: &str) -> Result<ComponentDeploymentConfigurationDigest, ConfigError> {
    let config = Config::parse_toml(source)?;
    Ok(config.compile_component_deployment_configuration_digest()?)
}

#[test]
fn semantic_digest_is_stable_across_source_order_and_formatting() {
    let left = r#"
[app]
name = "order_independent"
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
maximum_instances = 2
[component_specs.b]
component_role = "b"
maximum_instances = 2
[component_groups.cell.components.a]
component_spec = "a"
[component_groups.cell.components.b]
component_spec = "b"
[component_group_deployments.zeta]
component_group = "cell"
initial_placements = 0
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
[component_group_deployments.alpha]
component_group = "cell"
initial_placements = 0
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#;
    let right = r#"
# Comments and source order are not protected semantics.
[app]
name = "order_independent"

[roles.b]
package = "b"
kind = "canister"
[roles.a]
package = "a"
kind = "canister"
[roles.root]
package = "root"
kind = "root"

[component_specs.b]
maximum_instances = 2
component_role = "b"
[component_specs.a]
maximum_instances = 2
component_role = "a"

[component_groups.cell.components.b]
component_spec = "b"
[component_groups.cell.components.a]
component_spec = "a"

[component_group_deployments.alpha]
component_group = "cell"
maximum_placements = 1
initial_placements = 0
placement.minimum_distinct_roots = 1
placement.maximum_per_root = 1
[component_group_deployments.zeta]
component_group = "cell"
maximum_placements = 1
initial_placements = 0
placement.minimum_distinct_roots = 1
placement.maximum_per_root = 1
"#;

    assert_eq!(
        digest(left).expect("left digest"),
        digest(right).expect("right digest")
    );
}

#[test]
fn semantic_digest_changes_for_every_protected_projection() {
    let baseline = digest(BASELINE).expect("baseline digest");
    let mut swapped_purpose = BASELINE
        .replace(
            "service_purpose = \"authority\"",
            "service_purpose = \"pending\"",
        )
        .replace(
            "service_purpose = \"replica\"",
            "service_purpose = \"authority\"",
        )
        .replace(
            "service_purpose = \"pending\"",
            "service_purpose = \"replica\"",
        );
    swapped_purpose = swapped_purpose.replace(
        "authority_deployment = \"authoritative\"",
        "authority_deployment = \"replicas\"",
    );
    let changed = [
        BASELINE.replace("package = \"database\"", "package = \"database_v2\""),
        BASELINE
            .replace("components.database]", "components.primary_database]")
            .replace("member = [\"database\"]", "member = [\"primary_database\"]")
            .replace(
                "authority_member = [\"database\"]",
                "authority_member = [\"primary_database\"]",
            ),
        swapped_purpose,
        BASELINE.replace("zone = \"west\"", "zone = \"east\""),
        BASELINE.replace("maximum_instances_per_parent = 5", "maximum_instances_per_parent = 4"),
        BASELINE.replace(
            "component_group = \"apis\"\nservice_purpose = \"pool_member\"\ninitial_placements = 1",
            "component_group = \"apis\"\nservice_purpose = \"pool_member\"\ninitial_placements = 2",
        ),
        BASELINE.replace(
            "component_group = \"apis\"\nservice_purpose = \"pool_member\"\ninitial_placements = 1\nmaximum_placements = 2\nplacement.maximum_per_root = 2",
            "component_group = \"apis\"\nservice_purpose = \"pool_member\"\ninitial_placements = 1\nmaximum_placements = 2\nplacement.maximum_per_root = 1",
        ),
        BASELINE.replace(
            "mode = \"active_pool\"\nplacement.maximum_members_per_root = 2",
            "mode = \"active_pool\"\nplacement.maximum_members_per_root = 1",
        ),
    ];

    for source in changed {
        assert_ne!(digest(&source).expect("valid changed digest"), baseline);
    }
}

#[test]
fn equal_to_spec_reduction_has_one_semantic_digest() {
    let explicit = BASELINE.replace(
        "maximum_instances_per_parent = 5",
        "maximum_instances_per_parent = 10",
    );
    let omitted = explicit.replace(
        r#"
[[component_group_deployments.replicas.member_limits]]
member = ["database"]
spawn_grants = [
  { parent_role = "database", child_role = "child", maximum_instances_per_parent = 10 },
]
"#,
        "",
    );

    assert_eq!(
        digest(&explicit).expect("explicit equal-to-Spec digest"),
        digest(&omitted).expect("omitted equal-to-Spec digest")
    );
}

#[test]
fn canonical_configuration_digest_matches_schema_one_golden_vector() {
    assert_eq!(
        digest(BASELINE).expect("golden digest").to_string(),
        "5238d7e4d0e87a339d1fe358498b07b60ea57c511ef5c1ed02bb97b4cd55aa17"
    );
}
