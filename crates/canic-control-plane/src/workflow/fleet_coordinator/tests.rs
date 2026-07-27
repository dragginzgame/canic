//! Module: workflow::fleet_coordinator::tests
//!
//! Responsibility: qualify protected genesis commitment and canonical Coordinator queries.
//! Does not own: PocketIC installation or host effect-journal coverage.

use super::*;
use crate::storage::stable::fleet_coordinator::{
    FleetCoordinatorRegistryData, FleetCoordinatorRegistryStore,
};
use canic_core::{
    bootstrap::parse_config_model,
    dto::error::ErrorCode,
    ids::{
        AppId, CanonicalNetworkId, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
        FleetRegistryAuthority, SubnetId,
    },
};

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn init_args(coordinator: Principal) -> FleetCoordinatorInitArgs {
    let component_topology = parse_config_model(
        r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.project]
kind = "canister"
package = "project"

[component_specs.projects]
component_role = "project"
maximum_instances = 3
"#,
    )
    .expect("valid config")
    .compile_component_topology()
    .expect("Component Topology");
    FleetCoordinatorInitArgs {
        configured_app: AppId::from("demo"),
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::public_ic(),
                        fleet_id: FleetId::from_generated_bytes([7; 32]),
                    },
                    app: AppId::from("demo"),
                },
                coordinator_subnet: SubnetId::from_principal(principal(2)),
                coordinator,
            },
            epoch: 1,
        },
        component_topology,
    }
}

#[test]
fn protected_init_commits_exact_genesis_and_supports_exact_retry() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(3);
    let controller = principal(4);
    let args = init_args(coordinator);

    FleetCoordinatorWorkflow::initialize(args.clone(), controller, true, coordinator)
        .expect("commit genesis");
    FleetCoordinatorWorkflow::initialize(args, controller, true, coordinator)
        .expect("repeat exact genesis");

    let registry = FleetCoordinatorWorkflow::registry().expect("Registry");
    let manifest = FleetCoordinatorWorkflow::manifest().expect("manifest");
    let version = FleetCoordinatorWorkflow::version().expect("version");

    assert_eq!(registry.revision, 1);
    assert_eq!(registry.component_specs.len(), 1);
    assert!(registry.fleet_subnet_roots.is_empty());
    assert_eq!(manifest.revision, registry.revision);
    assert_eq!(version.content_hash, manifest.content_hash);

    let unauthorized = FleetCoordinatorWorkflow::initialize(
        init_args(coordinator),
        principal(5),
        false,
        coordinator,
    )
    .expect_err("reject non-controller init");
    assert_eq!(
        unauthorized.public_error().map(|error| error.code),
        Some(ErrorCode::Forbidden)
    );

    let wrong_canister = FleetCoordinatorWorkflow::initialize(
        init_args(principal(6)),
        controller,
        true,
        coordinator,
    )
    .expect_err("reject wrong Coordinator binding");
    assert_eq!(
        wrong_canister.public_error().map(|error| error.code),
        Some(ErrorCode::InvalidInput)
    );
}
