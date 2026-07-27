//! Module: pic::fleet_coordinator
//!
//! Responsibility: exercise the built-in Coordinator Registry lifecycle through PocketIC.
//! Does not own: host installation journals or root snapshot synchronization.
//! Boundary: builds one runtime-only Coordinator and calls its controller endpoint surface.

#[cfg(test)]
mod tests {
    use crate::pic::{CanicWasmBuildProfile, build_internal_test_wasm_canisters};
    use candid::{Principal, encode_one};
    use canic_control_plane::dto::fleet_coordinator::FleetCoordinatorInitArgs;
    use canic_core::{
        bootstrap::parse_config_model,
        cdk::types::Cycles,
        dto::{
            error::{Error, ErrorCode},
            fleet_registry::{
                FleetRegistry, FleetSubnetRootEntry, FleetSubnetRootJoinRequest,
                FleetSubnetRootJoinResponse, FleetSubnetRootStatus,
            },
        },
        ids::{
            AppId, CanonicalNetworkId, ComponentSpecAdmission, CyclesFundingBudget, FleetBinding,
            FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
            FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce,
            ReleaseSetDigest, SubnetId,
        },
        protocol,
    };
    use ic_testkit::{
        artifacts::{read_wasm, test_target_dir, workspace_root_for},
        pic::{PicBuilder, acquire_pic_serial_guard},
    };

    const COORDINATOR_PACKAGE: &str = "fleet_coordinator_stub";
    const INSTALL_CYCLES: u128 = 500_000_000_000_000;

    #[test]
    fn coordinator_commits_joining_roots_and_replays_original_receipts() {
        let _serial = acquire_pic_serial_guard();
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let target_dir = test_target_dir(&workspace_root, "fleet-coordinator");
        build_internal_test_wasm_canisters(
            &workspace_root,
            &target_dir,
            &[COORDINATOR_PACKAGE],
            CanicWasmBuildProfile::Fast,
        );
        let wasm = read_wasm(
            &target_dir,
            COORDINATOR_PACKAGE,
            CanicWasmBuildProfile::Fast.target_dir_name(),
        );
        let pic = PicBuilder::new().with_application_subnet().build();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, INSTALL_CYCLES);
        let args = init_args(coordinator);
        let topology = args.component_topology.clone();
        pic.install_canister(
            coordinator,
            wasm,
            encode_one(args).expect("encode Coordinator init"),
            None,
        );

        let genesis: Result<canic_core::dto::fleet_registry::FleetRegistryVersion, Error> = pic
            .query_call(coordinator, protocol::CANIC_FLEET_REGISTRY_VERSION, ())
            .expect("query genesis version");
        let genesis = genesis.expect("genesis version");
        let first_request = FleetSubnetRootJoinRequest {
            expected_registry: genesis,
            entry: joining_entry(&topology, 5, 21, 1),
        };
        let first: Result<FleetSubnetRootJoinResponse, Error> = pic
            .update_call(
                coordinator,
                protocol::CANIC_FLEET_SUBNET_ROOT_JOIN,
                (first_request.clone(),),
            )
            .expect("first join transport");
        let first = first.expect("first join");
        assert_eq!(first.version.revision, 2);

        let second_request = FleetSubnetRootJoinRequest {
            expected_registry: first.version.clone(),
            entry: joining_entry(&topology, 7, 22, 2),
        };
        let second: Result<FleetSubnetRootJoinResponse, Error> = pic
            .update_call(
                coordinator,
                protocol::CANIC_FLEET_SUBNET_ROOT_JOIN,
                (second_request,),
            )
            .expect("second join transport");
        let second = second.expect("second join");
        assert_eq!(second.version.revision, 3);

        let retried: Result<FleetSubnetRootJoinResponse, Error> = pic
            .update_call(
                coordinator,
                protocol::CANIC_FLEET_SUBNET_ROOT_JOIN,
                (first_request,),
            )
            .expect("late first retry transport");
        assert_eq!(
            retried.expect("late first retry"),
            first,
            "late exact retry must retain the original revision-two response"
        );

        let registry: Result<FleetRegistry, Error> = pic
            .query_call(coordinator, protocol::CANIC_FLEET_REGISTRY, ())
            .expect("query joined Registry");
        let registry = registry.expect("joined Registry");
        assert_eq!(registry.revision, 3);
        assert_eq!(registry.fleet_subnet_roots.len(), 2);

        let unauthorized: Result<FleetSubnetRootJoinResponse, Error> = pic
            .update_call_as(
                coordinator,
                principal(99),
                protocol::CANIC_FLEET_SUBNET_ROOT_JOIN,
                (FleetSubnetRootJoinRequest {
                    expected_registry: second.version,
                    entry: joining_entry(&topology, 9, 23, 1),
                },),
            )
            .expect("unauthorized join transport");
        assert_eq!(
            unauthorized
                .expect_err("non-controller join must fail")
                .code,
            ErrorCode::Unauthorized
        );
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

    fn joining_entry(
        topology: &canic_core::control_plane_support::config::ComponentTopology,
        subnet_byte: u8,
        root_byte: u8,
        maximum_root_instances: u32,
    ) -> FleetSubnetRootEntry {
        let spec = topology
            .component_specs
            .first()
            .expect("one Component Spec");
        let component_admissions = vec![ComponentSpecAdmission {
            component_spec: spec.component_spec.clone(),
            spec_hash: spec.spec_hash,
            maximum_root_instances,
        }];
        let component_topology_digest = topology
            .project_for_admissions(&component_admissions)
            .expect("root topology")
            .digest()
            .expect("root topology digest");
        FleetSubnetRootEntry {
            placement_subnet: SubnetId::from_principal(principal(subnet_byte)),
            fleet_subnet_root: principal(root_byte),
            component_admissions,
            component_topology_digest,
            active_release_set: FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [18; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([root_byte; 32]),
            },
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 3,
                maximum_managed_canisters: 100,
                maximum_registry_bytes: 2_097_152,
                maximum_wasm_store_bytes: 268_435_456,
                cycles_funding: CyclesFundingBudget {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(2_000_000_000_000),
                },
            },
            status: FleetSubnetRootStatus::Joining,
        }
    }

    fn principal(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }
}
