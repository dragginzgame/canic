//! Module: pic::fleet_coordinator
//!
//! Responsibility: exercise the built-in Coordinator Registry lifecycle through PocketIC.
//! Does not own: host installation journals or root-local snapshot staging.
//! Boundary: builds one Coordinator and calls its controller and registered-root endpoint surfaces.

#[cfg(test)]
mod tests {
    use crate::pic::artifacts::build_canonical_fleet_coordinator_wasm;
    use candid::{Principal, encode_one};
    use canic_control_plane::dto::fleet_coordinator::{
        CoordinatorCommand, CoordinatorCommandResponse, CoordinatorOperationStatusResponse,
        CoordinatorStatusRequest, CoordinatorStatusResponse, FleetCoordinatorInitArgs,
    };
    use canic_core::{
        bootstrap::parse_config_model,
        cdk::types::Cycles,
        control_plane_support::ops::{
            component_provisioning_plan::ComponentProvisioningPlanOps,
            fleet_registry::FleetRegistryOps,
        },
        dto::{
            authority_restore::{AuthorityRestoreFencePhase, AuthoritySnapshotRequest},
            component_provisioning::{
                ComponentGroupPlacementPlan, ComponentGroupPlanEntry,
                FleetComponentProvisioningOperation, FleetComponentProvisioningPhase,
                FleetComponentProvisioningPlan, FleetComponentProvisioningPrepareRequest,
                FleetSubnetRootProvisioningBatch,
            },
            error::Error,
            fleet_registry::{
                FleetRegistry, FleetRegistryActivationRequest, FleetRegistryActivationResponse,
                FleetSubnetRootEntry, FleetSubnetRootJoinRequest,
                FleetSubnetRootSnapshotAcknowledgementRequest, FleetSubnetRootStatus,
            },
            role::OperationStatusRequest,
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

    fn command(
        pic: &PocketIc,
        coordinator: Principal,
        command: CoordinatorCommand,
    ) -> Result<CoordinatorCommandResponse, Error> {
        pic.update_candid(coordinator, protocol::CANIC_COMMAND, (command,))
            .expect("Coordinator command transport")
    }

    fn command_as(
        pic: &PocketIc,
        coordinator: Principal,
        caller: Principal,
        command: CoordinatorCommand,
    ) -> Result<CoordinatorCommandResponse, Error> {
        pic.update_candid_as(coordinator, caller, protocol::CANIC_COMMAND, (command,))
            .expect("Coordinator command transport")
    }

    fn status(
        pic: &PocketIc,
        coordinator: Principal,
        request: CoordinatorStatusRequest,
    ) -> Result<CoordinatorStatusResponse, Error> {
        pic.query_candid(coordinator, protocol::CANIC_STATUS, (request,))
            .expect("Coordinator status transport")
    }

    fn status_as(
        pic: &PocketIc,
        coordinator: Principal,
        caller: Principal,
        request: CoordinatorStatusRequest,
    ) -> Result<CoordinatorStatusResponse, Error> {
        pic.query_candid_as(coordinator, caller, protocol::CANIC_STATUS, (request,))
            .expect("Coordinator status transport")
    }

    fn command_error(
        result: Result<CoordinatorCommandResponse, Error>,
        message: &'static str,
    ) -> Error {
        match result {
            Err(error) => error,
            Ok(_) => panic!("{message}"),
        }
    }

    fn status_error(
        result: Result<CoordinatorStatusResponse, Error>,
        message: &'static str,
    ) -> Error {
        match result {
            Err(error) => error,
            Ok(_) => panic!("{message}"),
        }
    }
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

        let CoordinatorStatusResponse::RegistryVersion(genesis) =
            status(&pic, coordinator, CoordinatorStatusRequest::RegistryVersion)
                .expect("genesis version")
        else {
            panic!("Coordinator returned a differently correlated status response");
        };
        let first_request = FleetSubnetRootJoinRequest {
            expected_registry: genesis,
            entry: joining_entry(&topology, 5, 21, 1),
        };
        let CoordinatorCommandResponse::JoinRoot(first) = command(
            &pic,
            coordinator,
            CoordinatorCommand::JoinRoot(first_request.clone()),
        )
        .expect("first join") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        assert_eq!(first.version.revision, 2);

        let second_request = FleetSubnetRootJoinRequest {
            expected_registry: first.version.clone(),
            entry: joining_entry(&topology, 7, 22, 2),
        };
        let CoordinatorCommandResponse::JoinRoot(second) = command(
            &pic,
            coordinator,
            CoordinatorCommand::JoinRoot(second_request),
        )
        .expect("second join") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        assert_eq!(second.version.revision, 3);

        let CoordinatorCommandResponse::JoinRoot(retried) = command(
            &pic,
            coordinator,
            CoordinatorCommand::JoinRoot(first_request),
        )
        .expect("late first retry") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        assert_eq!(
            retried, first,
            "late exact retry must retain the original revision-two response"
        );

        let CoordinatorStatusResponse::Registry(registry) =
            status(&pic, coordinator, CoordinatorStatusRequest::Registry).expect("joined Registry")
        else {
            panic!("Coordinator returned a differently correlated status response");
        };
        assert_eq!(registry.revision, 3);
        assert_eq!(registry.fleet_subnet_roots.len(), 2);

        assert_root_snapshot_endpoints(&pic, coordinator, &registry, &second.version);

        let unauthorized = command_as(
            &pic,
            coordinator,
            principal(99),
            CoordinatorCommand::JoinRoot(FleetSubnetRootJoinRequest {
                expected_registry: second.version,
                entry: joining_entry(&topology, 9, 23, 1),
            }),
        );
        assert_eq!(
            command_error(unauthorized, "non-controller join must fail").code(),
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
        let CoordinatorCommandResponse::OperationAccepted(receipt) = command(
            &pic,
            coordinator,
            CoordinatorCommand::ProvisionComponents(request),
        )
        .expect("prepare Component provisioning") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        assert_eq!(receipt.operation_id, [71; 32]);
        let CoordinatorStatusResponse::Operation(
            CoordinatorOperationStatusResponse::ComponentProvisioning(prepared),
        ) = status(
            &pic,
            coordinator,
            CoordinatorStatusRequest::Operation(OperationStatusRequest {
                operation_id: [71; 32],
            }),
        )
        .expect("Component provisioning status")
        else {
            panic!("Coordinator returned a differently correlated status response");
        };
        assert_eq!(prepared.phase, FleetComponentProvisioningPhase::Planned);
        assert_eq!(prepared.plan_hash, plan_hash);
        assert_eq!(prepared.root_batch_count, 2);
        assert_eq!(prepared.component_count, 2);

        let CoordinatorStatusResponse::Operation(
            CoordinatorOperationStatusResponse::ComponentProvisioning(observed),
        ) = status(
            &pic,
            coordinator,
            CoordinatorStatusRequest::Operation(OperationStatusRequest {
                operation_id: [71; 32],
            }),
        )
        .expect("Component provisioning status")
        else {
            panic!("Coordinator returned a differently correlated status response");
        };
        assert_eq!(observed, prepared);
    }

    fn assert_authority_snapshot_restore_fence(pic: &PocketIc, coordinator: Principal) {
        let request = AuthoritySnapshotRequest {
            operation_id: [41; 32],
        };
        let CoordinatorCommandResponse::PrepareAuthoritySnapshot(sealed) = command(
            pic,
            coordinator,
            CoordinatorCommand::PrepareAuthoritySnapshot(request),
        )
        .expect("authority snapshot prepare") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        assert_eq!(sealed.phase, AuthorityRestoreFencePhase::Sealed);
        assert_eq!(sealed.operation_id, Some(request.operation_id));

        let snapshots = pic
            .capture_controller_snapshots(coordinator, [coordinator])
            .expect("Coordinator snapshot capture");
        let CoordinatorCommandResponse::ResumeAuthoritySnapshot(resumed) = command(
            pic,
            coordinator,
            CoordinatorCommand::ResumeAuthoritySnapshot(request),
        )
        .expect("live authority snapshot resume") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        assert_eq!(resumed.phase, AuthorityRestoreFencePhase::Open);

        pic.restore_snapshots_with_captured_senders_and_funding(
            &snapshots,
            SnapshotRestoreFunding::TopUpTo {
                minimum_cycles: crate::pic::SNAPSHOT_RESTORE_MINIMUM_CYCLES,
            },
        )
        .expect("Coordinator snapshot restore");
        let CoordinatorStatusResponse::AuthorityRestore(restored) =
            status(pic, coordinator, CoordinatorStatusRequest::AuthorityRestore)
                .expect("restored authority fence status")
        else {
            panic!("Coordinator returned a differently correlated status response");
        };
        assert_eq!(restored.phase, AuthorityRestoreFencePhase::Sealed);

        let rejected_resume = command(
            pic,
            coordinator,
            CoordinatorCommand::ResumeAuthoritySnapshot(request),
        );
        assert_eq!(
            command_error(
                rejected_resume,
                "restored authority must remain mutation-fenced",
            )
            .code(),
            canic_core::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
        );
        let ordinary_mutation = command(
            pic,
            coordinator,
            CoordinatorCommand::ActivateRegistry(FleetRegistryActivationRequest {
                expected_registry: registry_version(pic, coordinator),
            }),
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
        let CoordinatorStatusResponse::RegistryVersion(version) =
            status(pic, coordinator, CoordinatorStatusRequest::RegistryVersion)
                .expect("restored Registry version")
        else {
            panic!("Coordinator returned a differently correlated status response");
        };
        version
    }

    fn assert_root_snapshot_endpoints(
        pic: &PocketIc,
        coordinator: Principal,
        registry: &FleetRegistry,
        version: &canic_core::dto::fleet_registry::FleetRegistryVersion,
    ) {
        let first_root = principal(21);
        let CoordinatorStatusResponse::Registry(snapshot) = status_as(
            pic,
            coordinator,
            first_root,
            CoordinatorStatusRequest::Registry,
        )
        .expect("registered root snapshot") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        assert_eq!(&snapshot, registry);

        let unregistered_snapshot = status_as(
            pic,
            coordinator,
            principal(99),
            CoordinatorStatusRequest::Registry,
        );
        assert_eq!(
            status_error(
                unregistered_snapshot,
                "unregistered root snapshot must fail",
            )
            .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );

        let request = FleetSubnetRootSnapshotAcknowledgementRequest {
            version: version.clone(),
        };
        let CoordinatorCommandResponse::AcknowledgeRootSnapshot(first_ack) = command_as(
            pic,
            coordinator,
            first_root,
            CoordinatorCommand::AcknowledgeRootSnapshot(request.clone()),
        )
        .expect("first acknowledgement") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        let CoordinatorCommandResponse::AcknowledgeRootSnapshot(repeated) = command_as(
            pic,
            coordinator,
            first_root,
            CoordinatorCommand::AcknowledgeRootSnapshot(request.clone()),
        )
        .expect("acknowledgement retry") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        assert_eq!(repeated, first_ack);
        let CoordinatorCommandResponse::AcknowledgeRootSnapshot(_) = command_as(
            pic,
            coordinator,
            principal(22),
            CoordinatorCommand::AcknowledgeRootSnapshot(request),
        )
        .expect("second acknowledgement") else {
            panic!("Coordinator returned a differently correlated command response");
        };

        let CoordinatorStatusResponse::RootAcknowledgements(acknowledgements) = status(
            pic,
            coordinator,
            CoordinatorStatusRequest::RootAcknowledgements,
        )
        .expect("acknowledgement inventory") else {
            panic!("Coordinator returned a differently correlated status response");
        };
        assert_eq!(acknowledgements.len(), 2);
        assert!(acknowledgements.iter().all(|ack| &ack.version == version));

        assert_registry_activation(pic, coordinator, version);
    }

    fn assert_registry_activation(
        pic: &PocketIc,
        coordinator: Principal,
        version: &canic_core::dto::fleet_registry::FleetRegistryVersion,
    ) -> FleetRegistryActivationResponse {
        let activation_request = FleetRegistryActivationRequest {
            expected_registry: version.clone(),
        };
        let CoordinatorCommandResponse::ActivateRegistry(activated) = command(
            pic,
            coordinator,
            CoordinatorCommand::ActivateRegistry(activation_request.clone()),
        )
        .expect("Registry activation") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        assert_eq!(&activated.previous_version, version);
        assert_eq!(activated.version.revision, version.revision + 1);
        let CoordinatorCommandResponse::ActivateRegistry(repeated) = command(
            pic,
            coordinator,
            CoordinatorCommand::ActivateRegistry(activation_request.clone()),
        )
        .expect("Registry activation retry") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        assert_eq!(repeated, activated);
        let unauthorized = command_as(
            pic,
            coordinator,
            principal(99),
            CoordinatorCommand::ActivateRegistry(activation_request),
        );
        assert_eq!(
            command_error(unauthorized, "non-controller activation must fail").code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
        );
        let CoordinatorStatusResponse::Registry(active) =
            status(pic, coordinator, CoordinatorStatusRequest::Registry)
                .expect("query active Registry")
        else {
            panic!("Coordinator returned a differently correlated status response");
        };
        assert!(
            active
                .fleet_subnet_roots
                .iter()
                .all(|entry| entry.status == FleetSubnetRootStatus::Active)
        );
        activated
    }

    fn activate_two_roots(
        pic: &PocketIc,
        coordinator: Principal,
        topology: &canic_core::control_plane_support::config::ComponentTopology,
    ) -> FleetRegistry {
        let CoordinatorStatusResponse::RegistryVersion(genesis) =
            status(pic, coordinator, CoordinatorStatusRequest::RegistryVersion)
                .expect("genesis version")
        else {
            panic!("Coordinator returned a differently correlated status response");
        };
        let CoordinatorCommandResponse::JoinRoot(first) = command(
            pic,
            coordinator,
            CoordinatorCommand::JoinRoot(FleetSubnetRootJoinRequest {
                expected_registry: genesis,
                entry: joining_entry(topology, 5, 21, 1),
            }),
        )
        .expect("first join") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        let CoordinatorCommandResponse::JoinRoot(second) = command(
            pic,
            coordinator,
            CoordinatorCommand::JoinRoot(FleetSubnetRootJoinRequest {
                expected_registry: first.version,
                entry: joining_entry(topology, 7, 22, 1),
            }),
        )
        .expect("second join") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        let joined_version = second.version;
        for root in [principal(21), principal(22)] {
            let CoordinatorCommandResponse::AcknowledgeRootSnapshot(_) = command_as(
                pic,
                coordinator,
                root,
                CoordinatorCommand::AcknowledgeRootSnapshot(
                    FleetSubnetRootSnapshotAcknowledgementRequest {
                        version: joined_version.clone(),
                    },
                ),
            )
            .expect("root acknowledgement") else {
                panic!("Coordinator returned a differently correlated command response");
            };
        }
        let CoordinatorCommandResponse::ActivateRegistry(_) = command(
            pic,
            coordinator,
            CoordinatorCommand::ActivateRegistry(FleetRegistryActivationRequest {
                expected_registry: joined_version,
            }),
        )
        .expect("activate Registry") else {
            panic!("Coordinator returned a differently correlated command response");
        };
        let CoordinatorStatusResponse::Registry(registry) =
            status(pic, coordinator, CoordinatorStatusRequest::Registry).expect("active Registry")
        else {
            panic!("Coordinator returned a differently correlated status response");
        };
        registry
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
