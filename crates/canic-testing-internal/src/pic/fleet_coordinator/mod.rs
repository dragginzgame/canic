//! Module: pic::fleet_coordinator
//!
//! Responsibility: exercise the built-in Coordinator Registry lifecycle through PocketIC.
//! Does not own: host installation journals or root-local snapshot staging.
//! Boundary: builds one Coordinator and calls its controller and registered-root endpoint surfaces.

#[cfg(test)]
pub(super) use tests::governed_pocketic_cases;

#[cfg(test)]
mod tests {
    use crate::pic::{artifacts::build_canonical_fleet_coordinator_wasm, startup::start_pocket_ic};
    use canic_contracts::protocol::initialization::encode;
    use canic_contracts::{
        cycles::Cycles,
        dto::{
            authority_restore::AuthoritySnapshotRequest,
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
        },
        ids::{
            AppId, CanonicalNetworkId, ComponentGroupPlacementId, ComponentSpecAdmission,
            CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
            FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding,
            FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce,
            ReleaseSetDigest, SubnetId,
        },
    };
    use canic_control_plane::installation::FleetCoordinatorInitArgs;
    use canic_core::{
        bootstrap::parse_config_model,
        control_plane_support::ops::{
            component_provisioning_plan::ComponentProvisioningPlanOps,
            fleet_registry::FleetRegistryOps,
        },
    };
    use ic_platform::principal::ExternalPrincipal as Principal;
    use {
        ic_platform::pocket_ic::{PocketIc, PocketIcBuilder},
        ic_testkit::pic::{CanisterSnapshotTarget, PocketIcSnapshotExt, SnapshotRestoreFunding},
    };

    const INSTALL_CYCLES: u128 = 500_000_000_000_000;

    fn acknowledge_root_snapshot(
        pic: &PocketIc,
        coordinator: Principal,
        root: Principal,
        request: FleetSubnetRootSnapshotAcknowledgementRequest,
    ) -> Result<canic_contracts::dto::fleet_registry::FleetSubnetRootSnapshotAcknowledgement, Error>
    {
        use canic_contracts::{
            dto::coordinator_acknowledgements, protocol::coordinator_acknowledgements as wire,
        };
        let response: Result<coordinator_acknowledgements::RootAcknowledgement, Error> =
            ic_testkit::pic::update_wire(
                pic,
                coordinator,
                root,
                ic_testkit::pic::WireCallSpec {
                    method: wire::acknowledge::METHOD,
                    method_id: wire::acknowledge::METHOD_ID,
                    request_limits: wire::acknowledge::REQUEST_LIMITS,
                    response_limits: wire::RESPONSE_LIMITS,
                },
                &coordinator_acknowledgements::AcknowledgeRequest::from(request),
            )
            .expect("native Root acknowledgement transport");
        response.map(Into::into)
    }

    fn command_error<T>(result: Result<T, Error>, message: &'static str) -> Error {
        match result {
            Err(error) => error,
            Ok(_) => panic!("{message}"),
        }
    }

    fn status_error<T>(result: Result<T, Error>, message: &'static str) -> Error {
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

    fn assert_public_health(pic: &PocketIc, coordinator: Principal) {
        let health: canic::dto::public_status::PublicHealth = ic_testkit::pic::query_wire(
            pic,
            coordinator,
            principal(99),
            ic_testkit::pic::WireCallSpec {
                method: canic::protocol::CANIC_PUBLIC_HEALTH,
                method_id: canic::protocol::public_health::METHOD_ID,
                request_limits: canic::protocol::public_health::REQUEST_LIMITS,
                response_limits: canic::protocol::public_health::RESPONSE_LIMITS,
            },
            &(),
        )
        .expect("public Wire health");
        let overview: Result<canic::dto::role::RoleOverviewResponse, Error> =
            ic_testkit::pic::query_wire(
                pic,
                coordinator,
                principal(99),
                ic_testkit::pic::WireCallSpec {
                    method: canic::protocol::CANIC_PUBLIC_OVERVIEW,
                    method_id: canic::protocol::public_overview::METHOD_ID,
                    request_limits: canic::protocol::public_overview::REQUEST_LIMITS,
                    response_limits: canic::protocol::public_overview::RESPONSE_LIMITS,
                },
                &(),
            )
            .unwrap();
        let overview = overview.unwrap();
        assert_eq!(health.canister_id, coordinator.into());
        assert_eq!(health.role.as_deref(), Some("fleet_coordinator"));
        assert_eq!(health.role.as_deref(), Some(overview.role.as_str()));
    }

    fn assert_supported_standards(pic: &PocketIc, coordinator: Principal) {
        let reply = pic
            .query_call(
                coordinator,
                principal(99),
                "icrc10_supported_standards",
                ic_platform::candid::contract::encode_args(()).expect("empty standards arguments"),
            )
            .expect("standards query is public for non-controllers");
        let (standards,): (Vec<(String, String)>,) =
            ic_platform::candid::decode_args(&reply, canic::testing::CANDID_REPLY_LIMITS)
                .expect("bounded standards reply");
        assert_eq!(
            standards,
            vec![(
                "ICRC-10".to_string(),
                "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-10".to_string(),
            )],
            "Coordinator must advertise its implemented standards only"
        );
    }

    #[test]
    fn coordinator_commits_joining_roots_and_replays_original_receipts() {
        let _unit_test_serial = super::super::acquire_pic_unit_test_serial_guard();
        let workspace_root =
            crate::workspace::discover().expect("discover Canic test workspace through Cargo");
        let wasm = build_canonical_fleet_coordinator_wasm(&workspace_root);
        let pic = start_pocket_ic(PocketIcBuilder::new().with_application_subnet());
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
            encode(args).expect("encode Coordinator init"),
            None,
        );

        assert_supported_standards(&pic, coordinator);
        assert_public_health(&pic, coordinator);

        let genesis =
            crate::pic::registry_discovery::version(&pic, coordinator).expect("genesis version");
        assert_registry_discovery(&pic, coordinator, &genesis);
        let first_request = FleetSubnetRootJoinRequest {
            expected_registry: genesis,
            entry: joining_entry(&topology, 5, 21, 1),
        };
        let first = crate::pic::registry_discovery::join(
            &pic,
            coordinator,
            Principal::anonymous(),
            first_request.clone(),
        )
        .expect("first join");
        assert_eq!(first.version.revision, 2);

        let second_request = FleetSubnetRootJoinRequest {
            expected_registry: first.version.clone(),
            entry: joining_entry(&topology, 7, 22, 2),
        };
        let second = crate::pic::registry_discovery::join(
            &pic,
            coordinator,
            Principal::anonymous(),
            second_request,
        )
        .expect("second join");
        assert_eq!(second.version.revision, 3);

        let retried = crate::pic::registry_discovery::join(
            &pic,
            coordinator,
            Principal::anonymous(),
            first_request,
        )
        .expect("late first retry");
        assert_eq!(
            retried, first,
            "late exact retry must retain the original revision-two response"
        );

        let registry =
            crate::pic::registry_discovery::snapshot(&pic, coordinator, Principal::anonymous())
                .expect("joined Registry");
        assert_eq!(registry.revision, 3);
        assert_eq!(registry.fleet_subnet_roots.len(), 2);

        assert_root_snapshot_endpoints(&pic, coordinator, &registry, &second.version);

        let unauthorized = crate::pic::registry_discovery::join(
            &pic,
            coordinator,
            principal(99),
            FleetSubnetRootJoinRequest {
                expected_registry: second.version,
                entry: joining_entry(&topology, 9, 23, 1),
            },
        );
        assert_eq!(
            command_error(unauthorized, "non-controller join must fail").code(),
            canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
        );

        assert_authority_snapshot_restore_fence(&pic, coordinator);
    }

    #[test]
    fn standalone_coordinator_prepares_from_its_durable_compiled_configuration() {
        let _unit_test_serial = super::super::acquire_pic_unit_test_serial_guard();
        let workspace_root =
            crate::workspace::discover().expect("discover Canic test workspace through Cargo");
        let wasm = build_canonical_fleet_coordinator_wasm(&workspace_root);
        let pic = start_pocket_ic(PocketIcBuilder::new().with_application_subnet());
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, INSTALL_CYCLES);
        let args = init_args(coordinator);
        let configuration = args.component_deployment_configuration.clone();
        pic.install_canister(
            coordinator,
            wasm,
            encode(args).expect("encode Coordinator init"),
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
        let receipt =
            crate::pic::batch_control::prepare(&pic, coordinator, Principal::anonymous(), request)
                .expect("prepare Component provisioning");
        assert_eq!(receipt.operation_id, [71; 32]);
        let prepared = crate::pic::canic::coordinator_provisioning(
            &pic,
            coordinator,
            Principal::anonymous(),
            [71; 32],
        )
        .expect("Component provisioning status");
        assert_eq!(prepared.phase, FleetComponentProvisioningPhase::Planned);
        assert_eq!(prepared.plan_hash, plan_hash);
        assert_eq!(prepared.root_batch_count, 2);
        assert_eq!(prepared.component_count, 2);

        let observed = crate::pic::canic::coordinator_provisioning(
            &pic,
            coordinator,
            Principal::anonymous(),
            [71; 32],
        )
        .expect("Component provisioning status");
        assert_eq!(observed, prepared);
    }

    fn assert_authority_snapshot_restore_fence(pic: &PocketIc, coordinator: Principal) {
        let request = AuthoritySnapshotRequest {
            operation_id: [41; 32],
        };
        let sealed = crate::pic::protected_state::prepare_snapshot(
            pic,
            coordinator,
            Principal::anonymous(),
            request,
        )
        .expect("authority snapshot prepare");
        assert_eq!(
            sealed.phase,
            canic::dto::authority_restore_status::Phase::Sealed
        );
        assert_eq!(sealed.operation_id, Some(request.operation_id));

        let snapshots = pic
            .capture_snapshots([CanisterSnapshotTarget {
                canister_id: coordinator,
                sender: None,
            }])
            .expect("Coordinator snapshot capture");
        let resumed = crate::pic::protected_state::resume_snapshot(
            pic,
            coordinator,
            Principal::anonymous(),
            request,
        )
        .expect("live authority snapshot resume");
        assert_eq!(
            resumed.phase,
            canic::dto::authority_restore_status::Phase::Open
        );

        pic.restore_snapshots(
            &snapshots,
            SnapshotRestoreFunding::TopUpTo {
                minimum_cycles: crate::pic::SNAPSHOT_RESTORE_MINIMUM_CYCLES,
            },
        )
        .expect("Coordinator snapshot restore");
        assert_public_health(pic, coordinator);
        let restored = crate::pic::protected_state::authority_restore(
            pic,
            coordinator,
            Principal::anonymous(),
        );
        let restored = restored.expect("restored Coordinator fence");
        assert_eq!(
            restored.phase,
            canic::dto::authority_restore_status::Phase::Sealed
        );
        assert_eq!(restored.authority_canister, coordinator.into());
        assert_eq!(restored.operation_id, Some(request.operation_id));
        let outsider =
            crate::pic::protected_state::authority_restore(pic, coordinator, principal(99));
        assert!(
            matches!(outsider, Err(error) if error.code() == canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code())
        );

        let rejected_resume = crate::pic::protected_state::resume_snapshot(
            pic,
            coordinator,
            Principal::anonymous(),
            request,
        );
        assert_eq!(
            status_error(
                rejected_resume,
                "restored authority must remain mutation-fenced",
            )
            .code(),
            canic_contracts::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
        );
        let ordinary_mutation = crate::pic::registry_discovery::activate(
            pic,
            coordinator,
            Principal::anonymous(),
            FleetRegistryActivationRequest {
                expected_registry: registry_version(pic, coordinator),
            },
        );
        assert!(
            ordinary_mutation.is_err(),
            "restored authority must reject ordinary mutation before handler dispatch"
        );
    }

    fn registry_version(
        pic: &PocketIc,
        coordinator: Principal,
    ) -> canic_contracts::dto::fleet_registry::FleetRegistryVersion {
        crate::pic::registry_discovery::version(pic, coordinator)
            .expect("restored Registry version")
    }

    fn assert_registry_discovery(
        pic: &PocketIc,
        coordinator: Principal,
        version: &canic_contracts::dto::fleet_registry::FleetRegistryVersion,
    ) {
        let manifest =
            crate::pic::registry_discovery::manifest(pic, coordinator, Principal::anonymous())
                .expect("registry manifest");
        assert_eq!(manifest.authority, version.authority);
        assert_eq!(manifest.revision, version.revision);
        assert_eq!(manifest.content_hash, version.content_hash);
        assert!(manifest.byte_length > 0);
        let rejected_manifest =
            crate::pic::registry_discovery::manifest(pic, coordinator, principal(99));
        let rejected_version =
            crate::pic::registry_discovery::version_as(pic, coordinator, principal(99));
        for error in [
            rejected_manifest.unwrap_err(),
            rejected_version.unwrap_err(),
        ] {
            assert_eq!(
                error.code(),
                canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
            );
        }
    }

    fn assert_root_snapshot_endpoints(
        pic: &PocketIc,
        coordinator: Principal,
        registry: &FleetRegistry,
        version: &canic_contracts::dto::fleet_registry::FleetRegistryVersion,
    ) {
        let first_root = principal(21);
        let snapshot = crate::pic::registry_discovery::snapshot(pic, coordinator, first_root)
            .expect("registered root snapshot");
        assert_eq!(&snapshot, registry);

        let unregistered_snapshot =
            crate::pic::registry_discovery::snapshot(pic, coordinator, principal(99));
        assert_eq!(
            status_error(
                unregistered_snapshot,
                "unregistered root snapshot must fail",
            )
            .code(),
            canic_contracts::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );

        let request = FleetSubnetRootSnapshotAcknowledgementRequest {
            version: version.clone(),
        };
        let first_ack = acknowledge_root_snapshot(pic, coordinator, first_root, request.clone())
            .expect("first acknowledgement");
        let repeated = acknowledge_root_snapshot(pic, coordinator, first_root, request.clone())
            .expect("acknowledgement retry");
        assert_eq!(repeated, first_ack);
        let _ = acknowledge_root_snapshot(pic, coordinator, principal(22), request)
            .expect("second acknowledgement");

        let acknowledgements = crate::pic::canic::coordinator_root_acknowledgements(
            pic,
            coordinator,
            Principal::anonymous(),
        )
        .expect("query Root acknowledgement inventory");
        let denied =
            crate::pic::canic::coordinator_root_acknowledgements(pic, coordinator, principal(99))
                .unwrap_err();
        assert_eq!(
            denied.code(),
            canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
        );
        assert_eq!(acknowledgements.len(), 2);
        assert!(acknowledgements.iter().all(|ack| &ack.version == version));

        assert_registry_activation(pic, coordinator, version);
    }

    fn assert_registry_activation(
        pic: &PocketIc,
        coordinator: Principal,
        version: &canic_contracts::dto::fleet_registry::FleetRegistryVersion,
    ) -> FleetRegistryActivationResponse {
        let activation_request = FleetRegistryActivationRequest {
            expected_registry: version.clone(),
        };
        let activated = crate::pic::registry_discovery::activate(
            pic,
            coordinator,
            Principal::anonymous(),
            activation_request.clone(),
        )
        .expect("Registry activation");
        assert_eq!(&activated.previous_version, version);
        assert_eq!(activated.version.revision, version.revision + 1);
        let repeated = crate::pic::registry_discovery::activate(
            pic,
            coordinator,
            Principal::anonymous(),
            activation_request.clone(),
        )
        .expect("Registry activation retry");
        assert_eq!(repeated, activated);
        let unauthorized = crate::pic::registry_discovery::activate(
            pic,
            coordinator,
            principal(99),
            activation_request,
        );
        assert_eq!(
            command_error(unauthorized, "non-controller activation must fail").code(),
            canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
        );
        let active =
            crate::pic::registry_discovery::snapshot(pic, coordinator, Principal::anonymous())
                .expect("query active Registry");
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
        let genesis =
            crate::pic::registry_discovery::version(pic, coordinator).expect("genesis version");
        let first = crate::pic::registry_discovery::join(
            pic,
            coordinator,
            Principal::anonymous(),
            FleetSubnetRootJoinRequest {
                expected_registry: genesis,
                entry: joining_entry(topology, 5, 21, 1),
            },
        )
        .expect("first join");
        let second = crate::pic::registry_discovery::join(
            pic,
            coordinator,
            Principal::anonymous(),
            FleetSubnetRootJoinRequest {
                expected_registry: first.version,
                entry: joining_entry(topology, 7, 22, 1),
            },
        )
        .expect("second join");
        let joined_version = second.version;
        for root in [principal(21), principal(22)] {
            let _ = acknowledge_root_snapshot(
                pic,
                coordinator,
                root,
                FleetSubnetRootSnapshotAcknowledgementRequest {
                    version: joined_version.clone(),
                },
            )
            .expect("root acknowledgement");
        }
        let _ = crate::pic::registry_discovery::activate(
            pic,
            coordinator,
            Principal::anonymous(),
            FleetRegistryActivationRequest {
                expected_registry: joined_version,
            },
        )
        .expect("activate Registry");

        crate::pic::registry_discovery::snapshot(pic, coordinator, Principal::anonymous())
            .expect("active Registry")
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
                    funding: root.funding.clone(),
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
        let fleet = FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([7; 32]),
            },
            app: AppId::from("demo"),
        };
        FleetCoordinatorInitArgs {
            configured_app: AppId::from("demo"),
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: fleet.clone(),
                    coordinator_subnet: SubnetId::from_principal(principal(2)),
                    coordinator,
                    recovery_controllers: Vec::new(),
                },
                epoch: 1,
            },
            admission: super::super::fleet_admission_policy(fleet),
            root_funding: Some(super::super::coordinator_root_funding_policy()),
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
                    creation_execution_margin: Cycles::new(1_000_000_000_000),
                },
                cycles_funding: CyclesFundingBudget {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(2_000_000_000_000),
                },
            },
            funding: super::super::root_funding_authority(),
            status: FleetSubnetRootStatus::Joining,
        }
    }

    fn principal(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }

    pub fn governed_pocketic_cases() -> Vec<crate::pic::GovernedTestCase> {
        crate::pic::cases::registered![
            (
                "Coordinator joining-root replay",
                coordinator_commits_joining_roots_and_replays_original_receipts,
            ),
            (
                "Coordinator durable compiled configuration",
                standalone_coordinator_prepares_from_its_durable_compiled_configuration,
            ),
        ]
    }
}
