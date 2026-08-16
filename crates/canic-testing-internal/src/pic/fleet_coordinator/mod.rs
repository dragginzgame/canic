//! Module: pic::fleet_coordinator
//!
//! Responsibility: exercise the built-in Coordinator Registry lifecycle through PocketIC.
//! Does not own: host installation journals or root-local snapshot staging.
//! Boundary: builds one Coordinator and calls its controller and registered-root endpoint surfaces.

#[cfg(test)]
mod tests {
    use crate::pic::artifacts::build_canonical_fleet_coordinator_wasm;
    use candid::{Principal, encode_one};
    use canic_control_plane::dto::fleet_coordinator::FleetCoordinatorInitArgs;
    use canic_core::{
        bootstrap::parse_config_model,
        cdk::types::Cycles,
        control_plane_support::ops::{
            component_provisioning_plan::ComponentProvisioningPlanOps,
            fleet_registry::FleetRegistryOps,
        },
        dto::{
            authority_restore::{
                AuthorityRestoreFencePhase, AuthorityRestoreFenceStatusResponse,
                AuthoritySnapshotRequest,
            },
            component_provisioning::{
                ComponentGroupPlacementPlan, ComponentGroupPlanEntry,
                FleetComponentProvisioningOperation, FleetComponentProvisioningPhase,
                FleetComponentProvisioningPlan, FleetComponentProvisioningPrepareRequest,
                FleetComponentProvisioningStatusRequest, FleetSubnetRootProvisioningBatch,
            },
            error::Error,
            fleet_registry::{
                FleetRegistry, FleetRegistryActivationRequest, FleetRegistryActivationResponse,
                FleetRegistrySnapshotResponse, FleetSubnetRootDrainingPublicationRequest,
                FleetSubnetRootDrainingPublicationResponse,
                FleetSubnetRootDrainingReservationRequest,
                FleetSubnetRootDrainingReservationResponse, FleetSubnetRootEntry,
                FleetSubnetRootJoinRequest, FleetSubnetRootJoinResponse,
                FleetSubnetRootRemovalPublicationRequest,
                FleetSubnetRootRemovalPublicationResponse, FleetSubnetRootSnapshotAcknowledgement,
                FleetSubnetRootSnapshotAcknowledgementRequest, FleetSubnetRootStatus,
            },
            fleet_subnet_root::{
                FleetSubnetRootDrainingResponse, FleetSubnetRootFinalInventoryResponse,
            },
        },
        ids::{
            AppId, CanonicalNetworkId, ComponentGroupPlacementId, ComponentSpecAdmission,
            CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
            FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding,
            FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce,
            ReleaseSetDigest, SubnetId,
        },
        protocol,
    };
    use ic_testkit::{
        artifacts::workspace_root_for,
        pic::{
            CandidCallExt, PocketIc, PocketIcBuilder, PocketIcCapturedSnapshotExt,
            PocketIcSnapshotExt, SnapshotRestoreFunding,
        },
    };

    const INSTALL_CYCLES: u128 = 500_000_000_000_000;
    const COORDINATOR_CONFIG: &str = r#"
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

[component_groups.project_cell.components.project]
component_spec = "projects"
service = "projects"

[component_group_deployments.project_cells]
component_group = "project_cell"
service_purpose = "pool_member"
initial_placements = 2
maximum_placements = 2
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 2

[services.fleet.targets.projects]
role = "project"
component_spec = "projects"
mode = "active_pool"
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 2
"#;

    #[test]
    fn coordinator_commits_joining_roots_and_replays_original_receipts() {
        let _unit_test_serial = super::super::acquire_pic_unit_test_serial_guard();
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let wasm = build_canonical_fleet_coordinator_wasm(&workspace_root);
        let pic = PocketIcBuilder::new().with_application_subnet().build();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, INSTALL_CYCLES);
        let args = init_args(coordinator);
        let topology = args
            .component_deployment_configuration
            .component_topology
            .clone();
        pic.install_canister(
            coordinator,
            wasm,
            encode_one(args).expect("encode Coordinator init"),
            None,
        );

        let genesis: Result<canic_core::dto::fleet_registry::FleetRegistryVersion, Error> = pic
            .query_candid(coordinator, protocol::CANIC_FLEET_REGISTRY_VERSION, ())
            .expect("query genesis version");
        let genesis = genesis.expect("genesis version");
        let first_request = FleetSubnetRootJoinRequest {
            expected_registry: genesis,
            entry: joining_entry(&topology, 5, 21, 1),
        };
        let first: Result<FleetSubnetRootJoinResponse, Error> = pic
            .update_candid(
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
            .update_candid(
                coordinator,
                protocol::CANIC_FLEET_SUBNET_ROOT_JOIN,
                (second_request,),
            )
            .expect("second join transport");
        let second = second.expect("second join");
        assert_eq!(second.version.revision, 3);

        let retried: Result<FleetSubnetRootJoinResponse, Error> = pic
            .update_candid(
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
            .query_candid(coordinator, protocol::CANIC_FLEET_REGISTRY, ())
            .expect("query joined Registry");
        let registry = registry.expect("joined Registry");
        assert_eq!(registry.revision, 3);
        assert_eq!(registry.fleet_subnet_roots.len(), 2);

        assert_root_snapshot_endpoints(&pic, coordinator, &registry, &second.version);

        let unauthorized: Result<FleetSubnetRootJoinResponse, Error> = pic
            .update_candid_as(
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
                .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
        );

        assert_authority_snapshot_restore_fence(&pic, coordinator);
    }

    #[test]
    fn standalone_coordinator_prepares_from_its_durable_compiled_configuration() {
        let _unit_test_serial = super::super::acquire_pic_unit_test_serial_guard();
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let wasm = build_canonical_fleet_coordinator_wasm(&workspace_root);
        let pic = PocketIcBuilder::new().with_application_subnet().build();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, INSTALL_CYCLES);
        let args = init_args(coordinator);
        let configuration = args.component_deployment_configuration.clone();
        pic.install_canister(
            coordinator,
            wasm,
            encode_one(args).expect("encode Coordinator init"),
            None,
        );

        let registry = activate_two_roots(&pic, coordinator, &configuration.component_topology);
        let plan = fresh_component_plan(&configuration, &registry);
        let plan_hash =
            ComponentProvisioningPlanOps::hash_compiled(&configuration, &registry, &plan)
                .expect("canonical plan hash");
        let request = FleetComponentProvisioningPrepareRequest {
            operation_id: [71; 32],
            plan,
        };
        let prepared: Result<
            canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
            Error,
        > = pic
            .update_candid(
                coordinator,
                protocol::CANIC_FLEET_COMPONENT_PROVISIONING_PREPARE,
                (request,),
            )
            .expect("prepare Component provisioning transport");
        let prepared = prepared.expect("prepare Component provisioning");
        assert_eq!(prepared.phase, FleetComponentProvisioningPhase::Planned);
        assert_eq!(prepared.plan_hash, plan_hash);
        assert_eq!(prepared.root_batch_count, 2);
        assert_eq!(prepared.component_count, 2);

        let observed: Result<
            canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
            Error,
        > = pic
            .query_candid(
                coordinator,
                protocol::CANIC_FLEET_COMPONENT_PROVISIONING_STATUS,
                (FleetComponentProvisioningStatusRequest {
                    operation_id: [71; 32],
                    plan_hash,
                },),
            )
            .expect("query Component provisioning status transport");
        assert_eq!(observed.expect("Component provisioning status"), prepared);
    }

    fn assert_authority_snapshot_restore_fence(pic: &PocketIc, coordinator: Principal) {
        let request = AuthoritySnapshotRequest {
            operation_id: [41; 32],
        };
        let sealed: Result<AuthorityRestoreFenceStatusResponse, Error> = pic
            .update_candid(
                coordinator,
                protocol::CANIC_AUTHORITY_SNAPSHOT_PREPARE,
                (request,),
            )
            .expect("authority snapshot prepare transport");
        let sealed = sealed.expect("authority snapshot prepare");
        assert_eq!(sealed.phase, AuthorityRestoreFencePhase::Sealed);
        assert_eq!(sealed.operation_id, Some(request.operation_id));

        let snapshots = pic
            .capture_controller_snapshots(coordinator, [coordinator])
            .expect("Coordinator snapshot capture");
        let resumed: Result<AuthorityRestoreFenceStatusResponse, Error> = pic
            .update_candid(
                coordinator,
                protocol::CANIC_AUTHORITY_SNAPSHOT_RESUME,
                (request,),
            )
            .expect("live authority snapshot resume transport");
        assert_eq!(
            resumed.expect("live authority snapshot resume").phase,
            AuthorityRestoreFencePhase::Open
        );

        pic.restore_snapshots_with_captured_senders_and_funding(
            &snapshots,
            SnapshotRestoreFunding::TopUpTo {
                minimum_cycles: crate::pic::SNAPSHOT_RESTORE_MINIMUM_CYCLES,
            },
        )
        .expect("Coordinator snapshot restore");
        let restored: Result<AuthorityRestoreFenceStatusResponse, Error> = pic
            .query_candid(
                coordinator,
                protocol::CANIC_AUTHORITY_RESTORE_FENCE_STATUS,
                (),
            )
            .expect("restored authority fence status transport");
        assert_eq!(
            restored.expect("restored authority fence status").phase,
            AuthorityRestoreFencePhase::Sealed
        );

        let rejected_resume: Result<AuthorityRestoreFenceStatusResponse, Error> = pic
            .update_candid(
                coordinator,
                protocol::CANIC_AUTHORITY_SNAPSHOT_RESUME,
                (request,),
            )
            .expect("restored authority snapshot resume transport");
        assert_eq!(
            rejected_resume
                .expect_err("restored authority must remain mutation-fenced")
                .code(),
            canic_core::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
        );
        let ordinary_mutation: Result<Result<FleetRegistryActivationResponse, Error>, _> = pic
            .update_candid(
                coordinator,
                protocol::CANIC_FLEET_REGISTRY_ACTIVATE,
                (FleetRegistryActivationRequest {
                    expected_registry: registry_version(pic, coordinator),
                },),
            );
        assert!(
            ordinary_mutation.is_err(),
            "restored authority must reject ordinary mutation before handler dispatch"
        );
    }

    fn registry_version(
        pic: &PocketIc,
        coordinator: Principal,
    ) -> canic_core::dto::fleet_registry::FleetRegistryVersion {
        let version: Result<canic_core::dto::fleet_registry::FleetRegistryVersion, Error> = pic
            .query_candid(coordinator, protocol::CANIC_FLEET_REGISTRY_VERSION, ())
            .expect("restored Registry version transport");
        version.expect("restored Registry version")
    }

    fn assert_root_snapshot_endpoints(
        pic: &PocketIc,
        coordinator: Principal,
        registry: &FleetRegistry,
        version: &canic_core::dto::fleet_registry::FleetRegistryVersion,
    ) {
        let first_root = principal(21);
        let snapshot: Result<FleetRegistrySnapshotResponse, Error> = pic
            .update_candid_as(
                coordinator,
                first_root,
                protocol::CANIC_FLEET_REGISTRY_SNAPSHOT_FOR_ROOT,
                (),
            )
            .expect("registered root snapshot transport");
        let snapshot = snapshot.expect("registered root snapshot");
        assert_eq!(&snapshot.registry, registry);
        assert_eq!(&snapshot.version, version);

        let unregistered_snapshot: Result<FleetRegistrySnapshotResponse, Error> = pic
            .update_candid_as(
                coordinator,
                principal(99),
                protocol::CANIC_FLEET_REGISTRY_SNAPSHOT_FOR_ROOT,
                (),
            )
            .expect("unregistered snapshot transport");
        assert_eq!(
            unregistered_snapshot
                .expect_err("unregistered root snapshot must fail")
                .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );

        let request = FleetSubnetRootSnapshotAcknowledgementRequest {
            version: version.clone(),
        };
        let first_ack: Result<FleetSubnetRootSnapshotAcknowledgement, Error> = pic
            .update_candid_as(
                coordinator,
                first_root,
                protocol::CANIC_FLEET_REGISTRY_ACKNOWLEDGE_ROOT,
                (request.clone(),),
            )
            .expect("first acknowledgement transport");
        let first_ack = first_ack.expect("first acknowledgement");
        let repeated: Result<FleetSubnetRootSnapshotAcknowledgement, Error> = pic
            .update_candid_as(
                coordinator,
                first_root,
                protocol::CANIC_FLEET_REGISTRY_ACKNOWLEDGE_ROOT,
                (request.clone(),),
            )
            .expect("acknowledgement retry transport");
        assert_eq!(repeated.expect("exact acknowledgement retry"), first_ack);
        let second_ack: Result<FleetSubnetRootSnapshotAcknowledgement, Error> = pic
            .update_candid_as(
                coordinator,
                principal(22),
                protocol::CANIC_FLEET_REGISTRY_ACKNOWLEDGE_ROOT,
                (request,),
            )
            .expect("second acknowledgement transport");
        second_ack.expect("second acknowledgement");

        let acknowledgements: Result<Vec<FleetSubnetRootSnapshotAcknowledgement>, Error> = pic
            .query_candid(
                coordinator,
                protocol::CANIC_FLEET_REGISTRY_ROOT_ACKNOWLEDGEMENTS,
                (),
            )
            .expect("acknowledgement inventory transport");
        let acknowledgements = acknowledgements.expect("acknowledgement inventory");
        assert_eq!(acknowledgements.len(), 2);
        assert!(acknowledgements.iter().all(|ack| &ack.version == version));

        let active = assert_registry_activation(pic, coordinator, version);
        assert_removed_root_snapshot_exclusion(pic, coordinator, registry, &active.version);
    }

    fn assert_registry_activation(
        pic: &PocketIc,
        coordinator: Principal,
        version: &canic_core::dto::fleet_registry::FleetRegistryVersion,
    ) -> FleetRegistryActivationResponse {
        let activation_request = FleetRegistryActivationRequest {
            expected_registry: version.clone(),
        };
        let activated: Result<FleetRegistryActivationResponse, Error> = pic
            .update_candid(
                coordinator,
                protocol::CANIC_FLEET_REGISTRY_ACTIVATE,
                (activation_request.clone(),),
            )
            .expect("Registry activation transport");
        let activated = activated.expect("Registry activation");
        assert_eq!(&activated.previous_version, version);
        assert_eq!(activated.version.revision, version.revision + 1);
        let repeated: Result<FleetRegistryActivationResponse, Error> = pic
            .update_candid(
                coordinator,
                protocol::CANIC_FLEET_REGISTRY_ACTIVATE,
                (activation_request.clone(),),
            )
            .expect("Registry activation retry transport");
        assert_eq!(
            repeated.expect("exact Registry activation retry"),
            activated
        );
        let unauthorized: Result<FleetRegistryActivationResponse, Error> = pic
            .update_candid_as(
                coordinator,
                principal(99),
                protocol::CANIC_FLEET_REGISTRY_ACTIVATE,
                (activation_request,),
            )
            .expect("unauthorized Registry activation transport");
        assert_eq!(
            unauthorized
                .expect_err("non-controller activation must fail")
                .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
        );
        let active: Result<FleetRegistry, Error> = pic
            .query_candid(coordinator, protocol::CANIC_FLEET_REGISTRY, ())
            .expect("query active Registry");
        assert!(
            active
                .expect("active Registry")
                .fleet_subnet_roots
                .iter()
                .all(|entry| entry.status == FleetSubnetRootStatus::Active)
        );
        activated
    }

    fn assert_removed_root_snapshot_exclusion(
        pic: &PocketIc,
        coordinator: Principal,
        joining_registry: &FleetRegistry,
        active_version: &canic_core::dto::fleet_registry::FleetRegistryVersion,
    ) {
        let removed_root = principal(21);
        let surviving_root = principal(22);
        let removed = publish_logical_root_removal(
            pic,
            coordinator,
            joining_registry,
            active_version,
            removed_root,
        );

        let rejected: Result<FleetRegistrySnapshotResponse, Error> = pic
            .update_candid_as(
                coordinator,
                removed_root,
                protocol::CANIC_FLEET_REGISTRY_SNAPSHOT_FOR_ROOT,
                (),
            )
            .expect("Removed root snapshot transport");
        assert_eq!(
            rejected
                .expect_err("Removed root must not remain a snapshot source")
                .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );
        let surviving: Result<FleetRegistrySnapshotResponse, Error> = pic
            .update_candid_as(
                coordinator,
                surviving_root,
                protocol::CANIC_FLEET_REGISTRY_SNAPSHOT_FOR_ROOT,
                (),
            )
            .expect("surviving root snapshot transport");
        let surviving = surviving.expect("surviving root snapshot");
        assert_eq!(surviving.version, removed.version);
        assert_eq!(
            surviving
                .registry
                .fleet_subnet_roots
                .iter()
                .find(|entry| entry.fleet_subnet_root == removed_root)
                .expect("Removed peer row")
                .status,
            FleetSubnetRootStatus::Removed
        );
    }

    fn publish_logical_root_removal(
        pic: &PocketIc,
        coordinator: Principal,
        joining_registry: &FleetRegistry,
        active_version: &canic_core::dto::fleet_registry::FleetRegistryVersion,
        removed_root: Principal,
    ) -> FleetSubnetRootRemovalPublicationResponse {
        let removed_entry = joining_registry
            .fleet_subnet_roots
            .iter()
            .find(|entry| entry.fleet_subnet_root == removed_root)
            .expect("removed root entry");
        let mut expected_root = removed_entry.clone();
        expected_root.status = FleetSubnetRootStatus::Active;
        let reservation: Result<FleetSubnetRootDrainingReservationResponse, Error> = pic
            .update_candid(
                coordinator,
                protocol::CANIC_FLEET_REGISTRY_ROOT_DRAINING_RESERVATION_PREPARE,
                (FleetSubnetRootDrainingReservationRequest {
                    operation_id: [31; 32],
                    expected_registry: active_version.clone(),
                    expected_root,
                },),
            )
            .expect("prepare root Draining reservation transport");
        let reservation = reservation.expect("prepare root Draining reservation");
        let draining: Result<FleetSubnetRootDrainingPublicationResponse, Error> = pic
            .update_candid(
                coordinator,
                protocol::CANIC_FLEET_REGISTRY_PUBLISH_ROOT_DRAINING,
                (FleetSubnetRootDrainingPublicationRequest {
                    expected_registry: active_version.clone(),
                    root_draining: FleetSubnetRootDrainingResponse {
                        operation_id: [31; 32],
                        fleet_subnet_root: removed_root,
                        placement_subnet: removed_entry.placement_subnet,
                        active_registry: active_version.clone(),
                        reservation_hash: reservation.reservation_hash,
                        component_topology_digest: removed_entry.component_topology_digest,
                        active_release_set: removed_entry.active_release_set,
                        next_allocation_sequence: 1,
                        reserved_component_instances: 0,
                        committed_component_instances: 0,
                        managed_descendants: 0,
                        known_created_component_canisters: 0,
                        root_registry_encoded_bytes: 0,
                        started_at_ns: 32,
                    },
                },),
            )
            .expect("publish root Draining transport");
        let draining = draining.expect("publish root Draining");
        let final_inventory = FleetSubnetRootFinalInventoryResponse {
            operation_id: [31; 32],
            fleet_subnet_root: removed_root,
            placement_subnet: removed_entry.placement_subnet,
            registry: draining.version.clone(),
            component_topology_digest: removed_entry.component_topology_digest,
            active_release_set: removed_entry.active_release_set,
            next_allocation_sequence: 1,
            removed_component_instances: 0,
            terminal_component_history_hash: [33; 32],
            root_registry_encoded_bytes: 0,
            wasm_store: principal(23),
            wasm_store_catalog_hash: [34; 32],
            wasm_store_catalog_entries: 1,
            wasm_store_occupied_bytes: 1_024,
            wasm_store_template_count: 1,
            wasm_store_release_count: 1,
            wasm_store_gc_prepared_at_secs: 35,
            finalized_at_ns: 36,
            inventory_hash: [37; 32],
        };
        let removed: Result<FleetSubnetRootRemovalPublicationResponse, Error> = pic
            .update_candid_as(
                coordinator,
                removed_root,
                protocol::CANIC_FLEET_REGISTRY_PUBLISH_ROOT_REMOVED,
                (FleetSubnetRootRemovalPublicationRequest {
                    expected_registry: draining.version,
                    final_inventory,
                },),
            )
            .expect("publish root Removed transport");
        removed.expect("publish root Removed")
    }

    fn activate_two_roots(
        pic: &PocketIc,
        coordinator: Principal,
        topology: &canic_core::control_plane_support::config::ComponentTopology,
    ) -> FleetRegistry {
        let genesis: Result<canic_core::dto::fleet_registry::FleetRegistryVersion, Error> = pic
            .query_candid(coordinator, protocol::CANIC_FLEET_REGISTRY_VERSION, ())
            .expect("query genesis version");
        let first: Result<FleetSubnetRootJoinResponse, Error> = pic
            .update_candid(
                coordinator,
                protocol::CANIC_FLEET_SUBNET_ROOT_JOIN,
                (FleetSubnetRootJoinRequest {
                    expected_registry: genesis.expect("genesis version"),
                    entry: joining_entry(topology, 5, 21, 1),
                },),
            )
            .expect("first join transport");
        let second: Result<FleetSubnetRootJoinResponse, Error> = pic
            .update_candid(
                coordinator,
                protocol::CANIC_FLEET_SUBNET_ROOT_JOIN,
                (FleetSubnetRootJoinRequest {
                    expected_registry: first.expect("first join").version,
                    entry: joining_entry(topology, 7, 22, 1),
                },),
            )
            .expect("second join transport");
        let joined_version = second.expect("second join").version;
        for root in [principal(21), principal(22)] {
            let acknowledgement: Result<FleetSubnetRootSnapshotAcknowledgement, Error> = pic
                .update_candid_as(
                    coordinator,
                    root,
                    protocol::CANIC_FLEET_REGISTRY_ACKNOWLEDGE_ROOT,
                    (FleetSubnetRootSnapshotAcknowledgementRequest {
                        version: joined_version.clone(),
                    },),
                )
                .expect("root acknowledgement transport");
            acknowledgement.expect("root acknowledgement");
        }
        let active: Result<FleetRegistryActivationResponse, Error> = pic
            .update_candid(
                coordinator,
                protocol::CANIC_FLEET_REGISTRY_ACTIVATE,
                (FleetRegistryActivationRequest {
                    expected_registry: joined_version,
                },),
            )
            .expect("activate Registry transport");
        active.expect("activate Registry");
        let registry: Result<FleetRegistry, Error> = pic
            .query_candid(coordinator, protocol::CANIC_FLEET_REGISTRY, ())
            .expect("query active Registry transport");
        registry.expect("active Registry")
    }

    fn fresh_component_plan(
        configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
        registry: &FleetRegistry,
    ) -> FleetComponentProvisioningPlan {
        let deployment = configuration
            .deployment_topology
            .get(&"project_cells".parse().expect("deployment ID"))
            .expect("project cells deployment");
        let entries = deployment
            .members
            .iter()
            .map(|member| ComponentGroupPlanEntry {
                member_path: member.member_path.clone(),
                component_spec: member.component_spec.clone(),
                spec_hash: member.component_spec_hash,
                purpose: member.purpose.clone(),
                labels: member.labels.clone(),
                limits: member.limits.clone(),
            })
            .collect::<Vec<_>>();
        let mut roots = registry.fleet_subnet_roots.iter().collect::<Vec<_>>();
        roots.sort_unstable_by_key(|root| root.fleet_subnet_root);
        let batches = roots
            .iter()
            .enumerate()
            .map(|(ordinal, root)| FleetSubnetRootProvisioningBatch {
                root: FleetSubnetRootBinding {
                    authority: registry.authority.clone(),
                    placement_subnet: root.placement_subnet,
                    fleet_subnet_root: root.fleet_subnet_root,
                    component_admissions: root.component_admissions.clone(),
                    component_topology_digest: root.component_topology_digest,
                    limits: root.limits.clone(),
                },
                active_release_set: root.active_release_set,
                placements: vec![ComponentGroupPlacementPlan {
                    group_placement: ComponentGroupPlacementId {
                        deployment: deployment.deployment.clone(),
                        ordinal: u32::try_from(ordinal).expect("placement ordinal"),
                    },
                    component_group: deployment.component_group.clone(),
                    entries: entries.clone(),
                }],
            })
            .collect();
        let directory_confirmation_roots =
            roots.iter().map(|root| root.fleet_subnet_root).collect();
        FleetComponentProvisioningPlan {
            fleet: registry.authority.binding.fleet.clone(),
            fleet_registry: FleetRegistryOps::version(
                &registry.authority,
                &configuration.component_topology,
                registry,
            )
            .expect("active Registry version"),
            configuration_digest: configuration.digest().expect("configuration digest"),
            operation: FleetComponentProvisioningOperation::FreshInstall,
            directory_confirmation_roots,
            batches,
        }
    }

    fn init_args(coordinator: Principal) -> FleetCoordinatorInitArgs {
        let component_deployment_configuration = parse_config_model(COORDINATOR_CONFIG)
            .expect("valid config")
            .compile_component_deployment_configuration()
            .expect("Component deployment configuration");
        FleetCoordinatorInitArgs {
            configured_app: AppId::from("demo"),
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                            fleet_id: FleetId::from_generated_bytes([7; 32]),
                        },
                        app: AppId::from("demo"),
                    },
                    coordinator_subnet: SubnetId::from_principal(principal(2)),
                    coordinator,
                },
                epoch: 1,
            },
            component_deployment_configuration,
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
                maximum_registry_bytes: 2_097_152,
                maximum_wasm_store_bytes: 268_435_456,
                maximum_group_placements: 16,
                canister_pool: FleetSubnetCanisterPoolConfig {
                    minimum_size: 1,
                    maximum_size: 10,
                    canister_cycles: Cycles::new(1_000_000_000_000),
                },
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
