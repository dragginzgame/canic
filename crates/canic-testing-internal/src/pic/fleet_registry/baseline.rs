//! Prepared-root Fleet Registry and Component Registry PocketIC journey.

use super::build::{SerialPic, build_pic, build_test_root_wasm, root_canister_config_path};
use candid::Principal;
use ic_testkit::pic::Pic;
use std::path::Path;

const ROOT_INSTALL_CYCLES: u128 = 80_000_000_000_000;

mod tests {
    use super::*;
    use candid::{decode_one, encode_one};
    use canic::{
        CANIC_WASM_CHUNK_BYTES,
        dto::{
            component_registry::{
                ComponentDirectoryHead, ComponentDirectoryHeadRequest, ComponentLifecycleStatus,
                ComponentProvisioningOrigin, ComponentRegistryPartitionRequest,
                ComponentRegistryPartitionResponse, ComponentRuntimePhase,
                ComponentRuntimeStatusResponse, RootComponentAllocationPhase,
                RootComponentAllocationRequest, RootComponentAllocationResponse,
                RootComponentAllocationStatusRequest, RootComponentChildAllocationRequest,
                RootComponentChildAllocationResponse, RootComponentChildCommitRequest,
                RootComponentChildCommitResponse, RootComponentChildCreationRequest,
                RootComponentChildDirectoryPreparationRequest,
                RootComponentChildDirectoryPreparationResponse, RootComponentChildInstallRequest,
                RootComponentChildMembershipActivationRequest,
                RootComponentChildMembershipActivationResponse,
                RootComponentChildRuntimeActivationRequest,
                RootComponentChildRuntimeActivationResponse, RootComponentCommitRequest,
                RootComponentCommitResponse, RootComponentCreationRequest,
                RootComponentDirectoryPreparationRequest,
                RootComponentDirectoryPreparationResponse, RootComponentInitialInventoryStatus,
                RootComponentInstallRequest, RootComponentMembershipActivationRequest,
                RootComponentMembershipActivationResponse, RootComponentRegistryPreparationRequest,
                RootComponentRegistryStatusResponse, RootComponentRuntimeActivationRequest,
                RootComponentRuntimeActivationResponse,
            },
            fleet_registry::{
                FleetDirectoryProvenance, FleetDirectorySnapshot, FleetRegistry,
                FleetRegistryActivationRequest, FleetRegistryActivationResponse,
                FleetSubnetRootDirectoryEntry, FleetSubnetRootEntry, FleetSubnetRootJoinRequest,
                FleetSubnetRootJoinResponse, FleetSubnetRootRegistryMirrorActivationRequest,
                FleetSubnetRootRegistryMirrorActivationResponse,
                FleetSubnetRootRegistrySyncRequest, FleetSubnetRootRegistrySyncResponse,
                FleetSubnetRootSnapshotAcknowledgement, FleetSubnetRootStatus,
            },
            fleet_subnet_root::{
                FleetSubnetRootAuthority, FleetSubnetRootCanisterSummary, FleetSubnetRootInitArgs,
            },
            root_store::{
                ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX, ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX,
                RootStoreArtifact, RootStoreBootstrapRequest, RootStoreBootstrapResponse,
                RootStoreReleaseSetEntry, RootStoreReleaseSetEntryKind,
                RootStoreReleaseSetManifest,
            },
        },
        ids::{
            CanisterRole, ComponentBinding, ComponentInstanceId, ManagedCanisterBinding,
            ReleaseSetDigest,
        },
    };
    use canic::{
        Error,
        dto::fleet_activation::{
            FleetActivationPhase, FleetActivationResumeRequest, FleetActivationStatusResponse,
        },
        protocol::{
            CANIC_COMPONENT_RUNTIME_STATUS, CANIC_FLEET_ACTIVATION_STATUS, CANIC_FLEET_REGISTRY,
            CANIC_FLEET_REGISTRY_ACTIVATE, CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
            CANIC_FLEET_REGISTRY_MIRROR_STATUS, CANIC_FLEET_REGISTRY_ROOT_ACKNOWLEDGEMENTS,
            CANIC_FLEET_REGISTRY_SYNC_STATUS, CANIC_FLEET_REGISTRY_SYNCHRONIZE,
            CANIC_FLEET_REGISTRY_VERSION, CANIC_FLEET_SUBNET_ROOT_AUTHORITY,
            CANIC_FLEET_SUBNET_ROOT_CANISTER_SUMMARY, CANIC_FLEET_SUBNET_ROOT_JOIN,
            CANIC_PREPARE_FLEET_ACTIVATION, CANIC_RESUME_FLEET_ACTIVATION,
            CANIC_ROOT_COMPONENT_ALLOCATE, CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
            CANIC_ROOT_COMPONENT_CHILD_ALLOCATE, CANIC_ROOT_COMPONENT_CHILD_COMMIT,
            CANIC_ROOT_COMPONENT_CHILD_CREATE, CANIC_ROOT_COMPONENT_CHILD_DIRECTORY_PREPARE,
            CANIC_ROOT_COMPONENT_CHILD_INSTALL, CANIC_ROOT_COMPONENT_CHILD_MEMBERSHIP_ACTIVATE,
            CANIC_ROOT_COMPONENT_CHILD_RUNTIME_ACTIVATE, CANIC_ROOT_COMPONENT_COMMIT,
            CANIC_ROOT_COMPONENT_CREATE, CANIC_ROOT_COMPONENT_DIRECTORY_HEAD,
            CANIC_ROOT_COMPONENT_DIRECTORY_PREPARE, CANIC_ROOT_COMPONENT_INSTALL,
            CANIC_ROOT_COMPONENT_MEMBERSHIP_ACTIVATE, CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
            CANIC_ROOT_COMPONENT_REGISTRY_PREPARE, CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
            CANIC_ROOT_COMPONENT_RUNTIME_ACTIVATE, CANIC_ROOT_STORE_BOOTSTRAP,
            CANIC_TEMPLATE_PREPARE_ADMIN, CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
            CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
        },
    };
    use canic_control_plane::{
        dto::fleet_coordinator::FleetCoordinatorInitArgs,
        dto::template::{
            TemplateChunkInput, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
            TemplateManifestInput,
        },
        ids::{
            TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion,
            WasmStoreBinding,
        },
    };
    use canic_core::cdk::utils::hash::{hex_bytes, wasm_hash};
    use canic_host::release_set::AppConfigSnapshot;
    use flate2::{Compression, write::GzEncoder};
    use std::{collections::BTreeMap, io::Write, sync::OnceLock};

    use crate::pic::{
        CanicWasmBuildProfile,
        artifacts::build_internal_test_wasm_canisters_with_env,
        build_internal_test_wasm_canisters,
        canic::{
            install_root_args_with_release_set_digest_and_coordinator, managed_test_init_identity,
        },
    };
    use ic_testkit::artifacts::{read_wasm, test_target_dir, workspace_root_for};

    #[cfg(test)]
    use canic::protocol::{CANIC_ROOT_STORE_BOOTSTRAP_STATUS, CANIC_WASM_STORE_PREPARE};
    #[cfg(test)]
    use canic::{
        dto::component_registry::{
            RootComponentSubtreeRemovalAdvanceRequest, RootComponentSubtreeRemovalPhase,
            RootComponentSubtreeRemovalRequest, RootComponentSubtreeRemovalResponse,
            RootComponentSubtreeRemovalStatusRequest,
            RootComponentSubtreeRemovalStopPreparationRequest,
            RootComponentSubtreeRemovalStopRequest,
        },
        protocol::{
            CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_ADVANCE,
            CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_BEGIN,
            CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STATUS, CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP,
            CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP_PREPARE,
        },
    };

    const COORDINATOR_PACKAGE: &str = "fleet_coordinator_stub";
    const ISSUER_PACKAGE: &str = "delegation_issuer_stub";
    const PROJECT_HUB_PACKAGE: &str = "project_hub_stub";
    const PROJECT_INSTANCE_PACKAGE: &str = "project_instance_stub";
    const COORDINATOR_INSTALL_CYCLES: u128 = 500_000_000_000_000;

    ///
    /// ActiveComponentRegistryFixture
    ///
    /// Fresh Coordinator-anchored Fleet fixture whose root and two Components
    /// are active under current Component Registry authority.
    ///
    pub struct ActiveComponentRegistryFixture {
        pic: SerialPic,
        pub coordinator: Principal,
        pub root: Principal,
        pub issuer: ComponentBinding,
        pub verifier: ComponentBinding,
    }

    impl ActiveComponentRegistryFixture {
        /// Borrow the live PocketIC instance while retaining its serial guard.
        #[must_use]
        pub const fn pic(&self) -> &Pic {
            &self.pic.pic
        }
    }

    struct ActiveComponentBindings {
        issuer: ComponentBinding,
        verifier: ComponentBinding,
    }

    impl ActiveComponentBindings {
        const fn new(issuer: ComponentBinding, verifier: ComponentBinding) -> Self {
            Self { issuer, verifier }
        }
    }

    struct BootstrappedRootFixture {
        root_id: Principal,
        init_args: FleetSubnetRootInitArgs,
        request: RootStoreBootstrapRequest,
        response: RootStoreBootstrapResponse,
    }

    struct RootStoreFixture {
        manifest: RootStoreReleaseSetManifest,
        artifacts: BTreeMap<CanisterRole, Vec<u8>>,
    }

    #[test]
    fn prepared_root_bootstraps_and_reverifies_its_exact_local_store() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let root_wasm = build_test_root_wasm();
        let store_fixture = build_root_store_fixture();
        let pic = build_pic();
        let fixture = install_bootstrapped_root(
            &pic,
            root_wasm,
            Principal::from_slice(&[0x41; 29]),
            store_fixture,
        );

        assert_eq!(fixture.response.fleet_subnet_root, fixture.root_id);
        assert_eq!(
            fixture.response.release_set,
            fixture.init_args.authority.initial_release_set
        );
        assert_eq!(fixture.response.catalog.len(), 3);

        let retried: Result<RootStoreBootstrapResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_STORE_BOOTSTRAP,
                (fixture.request.clone(),),
            )
            .expect("root Store bootstrap retry transport");
        assert_eq!(
            retried.expect("root Store bootstrap retry"),
            fixture.response,
            "exact update retry must return the same Store evidence"
        );
        let observed: Result<RootStoreBootstrapResponse, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_ROOT_STORE_BOOTSTRAP_STATUS,
                (fixture.request,),
            )
            .expect("root Store status transport");
        assert_eq!(
            observed.expect("root Store status"),
            fixture.response,
            "composite status must independently reverify the exact live catalog"
        );

        let payload = b"direct root Store authorization";
        let payload_hash = wasm_hash(payload);
        let prepare = TemplateChunkSetPrepareInput {
            template_id: TemplateId::owned("canary:direct-root-update".to_string()),
            version: TemplateVersion::from(format!(
                "{}-direct-root-update",
                env!("CARGO_PKG_VERSION")
            )),
            payload_hash: payload_hash.clone(),
            payload_size_bytes: payload.len() as u64,
            chunk_hashes: vec![payload_hash],
        };
        let prepared: Result<TemplateChunkSetInfoResponse, Error> = pic
            .update_call_as(
                fixture.response.wasm_store,
                fixture.root_id,
                CANIC_WASM_STORE_PREPARE,
                (prepare.clone(),),
            )
            .expect("direct root Store prepare transport");
        assert_eq!(
            prepared.expect("direct root Store prepare").chunk_hashes,
            prepare.chunk_hashes
        );

        let denied: Result<TemplateChunkSetInfoResponse, Error> = pic
            .update_call_as(
                fixture.response.wasm_store,
                Principal::anonymous(),
                CANIC_WASM_STORE_PREPARE,
                (prepare,),
            )
            .expect("anonymous Store prepare transport");
        assert_eq!(
            denied.expect_err("anonymous Store prepare must fail").code,
            canic::dto::error::ErrorCode::Unauthorized
        );
        assert_prepared(&pic, fixture.root_id);
    }

    #[test]
    fn prepared_root_activates_only_after_complete_initial_component_inventory() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_active_component_registry();
        super::super::role_attestation::assert_registry_bound_role_attestation(
            fixture.pic(),
            fixture.root,
            &fixture.issuer,
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one PocketIC journey keeps the complete fence-and-stop boundary coherent"
    )]
    fn active_root_stops_one_exact_registered_child_before_removal() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_active_component_registry();
        let (child, registry) = create_active_project_instance(&fixture);
        let operation_id = [0xd1; 32];
        let begin_request = RootComponentSubtreeRemovalRequest {
            operation_id,
            component: fixture.verifier.component,
            target_canister_id: child,
            expected_registry: registry,
        };
        let fenced: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .update_call(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_BEGIN,
                (begin_request,),
            )
            .expect("begin subtree removal transport");
        assert_eq!(
            fenced.expect("begin subtree removal").phase,
            RootComponentSubtreeRemovalPhase::Fenced
        );

        let advanced: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .update_call(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_ADVANCE,
                (RootComponentSubtreeRemovalAdvanceRequest {
                    operation_id,
                    component: fixture.verifier.component,
                    expected_traversal_steps: 0,
                },),
            )
            .expect("advance subtree removal transport");
        let advanced = advanced.expect("advance subtree removal");
        assert_eq!(advanced.traversal_steps, 1);
        assert!(matches!(
            &advanced.phase,
            RootComponentSubtreeRemovalPhase::LeafSelected(leaf)
                if leaf.canister_id == child
                    && leaf.parent_canister_id == fixture.verifier.canister_id
        ));

        let stop_request = RootComponentSubtreeRemovalStopRequest {
            operation_id,
            component: fixture.verifier.component,
            expected_traversal_steps: advanced.traversal_steps,
            expected_leaf_canister_id: child,
            expected_leaf_parent_canister_id: fixture.verifier.canister_id,
        };
        let prepared: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .update_call(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP_PREPARE,
                (RootComponentSubtreeRemovalStopPreparationRequest {
                    operation_id: stop_request.operation_id,
                    component: stop_request.component,
                    expected_traversal_steps: stop_request.expected_traversal_steps,
                    expected_leaf_canister_id: stop_request.expected_leaf_canister_id,
                    expected_leaf_parent_canister_id: stop_request.expected_leaf_parent_canister_id,
                },),
            )
            .expect("prepare subtree leaf stop transport");
        assert!(matches!(
            prepared.expect("prepare subtree leaf stop").phase,
            RootComponentSubtreeRemovalPhase::StopIntent(_)
        ));

        let stopped: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .update_call(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP,
                (stop_request,),
            )
            .expect("stop subtree leaf transport");
        let stopped = stopped.expect("stop subtree leaf");
        let RootComponentSubtreeRemovalPhase::Stopped(receipt) = &stopped.phase else {
            panic!("subtree leaf must retain an independently observed stopped receipt");
        };
        assert_eq!(receipt.stop.leaf.canister_id, child);
        assert_eq!(
            receipt.stop.leaf.parent_canister_id,
            fixture.verifier.canister_id
        );
        assert_eq!(receipt.stop.controller, fixture.root);
        assert_ne!(receipt.observed_module_hash, [0; 32]);
        let live = fixture
            .pic()
            .canister_status(child, Some(fixture.root))
            .expect("stopped child status");
        assert_eq!(
            format!("{:?}", live.status),
            "Stopped",
            "the independently receipted child must be stopped in PocketIC"
        );
        assert_eq!(live.settings.controllers, vec![fixture.root]);
        assert_eq!(
            live.module_hash.as_deref(),
            Some(receipt.observed_module_hash.as_slice())
        );

        let retry: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .update_call(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP,
                (stop_request,),
            )
            .expect("retry stopped subtree leaf transport");
        assert_eq!(retry.expect("retry stopped subtree leaf"), stopped);

        let durable: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .query_call(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STATUS,
                (RootComponentSubtreeRemovalStatusRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("query stopped subtree receipt transport");
        assert_eq!(durable.expect("query stopped subtree receipt"), stopped);
    }

    /// Build one current Coordinator/root/Store fixture with active Registry-issued Components.
    ///
    /// # Panics
    ///
    /// Panics when a fixture artifact cannot be built or any required PocketIC
    /// management, Registry, Store, allocation, installation, or activation call
    /// fails its current protocol contract.
    #[must_use]
    pub fn setup_active_component_registry() -> ActiveComponentRegistryFixture {
        let root_wasm = build_test_root_wasm();
        let coordinator_wasm = build_test_coordinator_wasm();
        let store_fixture = build_root_store_fixture();
        let pic = build_pic();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let fixture = install_bootstrapped_root(&pic, root_wasm, coordinator, store_fixture);
        install_fixture_coordinator(&pic, coordinator, coordinator_wasm, &fixture);

        let genesis: Result<canic::dto::fleet_registry::FleetRegistryVersion, Error> = pic
            .query_call(coordinator, CANIC_FLEET_REGISTRY_VERSION, ())
            .expect("query Registry genesis");
        let genesis = genesis.expect("Registry genesis");
        let binding = &fixture.init_args.authority.binding;
        let join_request = FleetSubnetRootJoinRequest {
            expected_registry: genesis,
            entry: FleetSubnetRootEntry {
                placement_subnet: binding.placement_subnet,
                fleet_subnet_root: fixture.root_id,
                component_admissions: binding.component_admissions.clone(),
                component_topology_digest: binding.component_topology_digest,
                active_release_set: fixture.init_args.authority.initial_release_set,
                limits: binding.limits.clone(),
                status: FleetSubnetRootStatus::Joining,
            },
        };
        let joined: Result<FleetSubnetRootJoinResponse, Error> = pic
            .update_call(coordinator, CANIC_FLEET_SUBNET_ROOT_JOIN, (join_request,))
            .expect("join root transport");
        let joined = joined.expect("join root");

        let sync_request = FleetSubnetRootRegistrySyncRequest {
            expected_registry: joined.version.clone(),
            store_bootstrap: fixture.request.clone(),
        };
        let synchronized: Result<FleetSubnetRootRegistrySyncResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_SYNCHRONIZE,
                (sync_request.clone(),),
            )
            .expect("root Registry synchronization transport");
        let synchronized = synchronized.expect("root Registry synchronization");
        assert_eq!(synchronized.fleet_subnet_root, fixture.root_id);
        assert_eq!(synchronized.version, joined.version);

        let retried: Result<FleetSubnetRootRegistrySyncResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_SYNCHRONIZE,
                (sync_request.clone(),),
            )
            .expect("root Registry synchronization retry transport");
        assert_eq!(
            retried.expect("root Registry synchronization retry"),
            synchronized
        );
        let observed: Result<FleetSubnetRootRegistrySyncResponse, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_SYNC_STATUS,
                (sync_request.clone(),),
            )
            .expect("root Registry synchronization status transport");
        assert_eq!(
            observed.expect("root Registry synchronization status"),
            synchronized
        );

        let acknowledgements: Result<Vec<FleetSubnetRootSnapshotAcknowledgement>, Error> = pic
            .query_call(coordinator, CANIC_FLEET_REGISTRY_ROOT_ACKNOWLEDGEMENTS, ())
            .expect("query root acknowledgements");
        assert_eq!(
            acknowledgements.expect("root acknowledgements"),
            vec![synchronized.acknowledgement]
        );

        let components = assert_registry_and_root_runtime_activation(
            &pic,
            coordinator,
            &fixture,
            joined.version,
            sync_request,
        );
        let fixture = ActiveComponentRegistryFixture {
            pic,
            coordinator,
            root: fixture.root_id,
            issuer: components.issuer,
            verifier: components.verifier,
        };
        assert_root_canister_summary(&fixture);
        fixture
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "used only by the focused PocketIC removal test")
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "the fixture must drive every real child lifecycle phase before removal"
    )]
    fn create_active_project_instance(
        fixture: &ActiveComponentRegistryFixture,
    ) -> (
        Principal,
        canic::dto::component_registry::ComponentRegistryHead,
    ) {
        let partition: Result<ComponentRegistryPartitionResponse, Error> = fixture
            .pic()
            .query_call(
                fixture.root,
                CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
                (ComponentRegistryPartitionRequest {
                    component: fixture.verifier.component,
                },),
            )
            .expect("query projects Component Registry transport");
        let partition = partition.expect("query projects Component Registry");
        let operation_id = [0xc1; 32];
        let reserved: Result<RootComponentChildAllocationResponse, Error> = fixture
            .pic()
            .update_call_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_ALLOCATE,
                (RootComponentChildAllocationRequest {
                    operation_id,
                    component: fixture.verifier.component,
                    expected_registry: partition.head,
                    child_role: CanisterRole::new("project_instance"),
                },),
            )
            .expect("reserve project instance transport");
        assert_eq!(
            reserved.expect("reserve project instance").phase,
            RootComponentAllocationPhase::Reserved
        );

        let created: Result<RootComponentChildAllocationResponse, Error> = fixture
            .pic()
            .update_call_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_CREATE,
                (RootComponentChildCreationRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("create project instance transport");
        let created = created.expect("create project instance");
        assert_eq!(created.phase, RootComponentAllocationPhase::Created);
        let child = created
            .creation
            .as_ref()
            .and_then(|creation| creation.canister)
            .expect("created project instance Canister");

        let installed: Result<RootComponentChildAllocationResponse, Error> = fixture
            .pic()
            .update_call_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_INSTALL,
                (RootComponentChildInstallRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("install project instance transport");
        assert_eq!(
            installed.expect("install project instance").phase,
            RootComponentAllocationPhase::Verified
        );

        let committed: Result<RootComponentChildCommitResponse, Error> = fixture
            .pic()
            .update_call_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_COMMIT,
                (RootComponentChildCommitRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("commit project instance transport");
        let committed = committed.expect("commit project instance");
        assert_eq!(
            committed.allocation.phase,
            RootComponentAllocationPhase::Committed
        );

        let prepared: Result<RootComponentChildDirectoryPreparationResponse, Error> = fixture
            .pic()
            .update_call_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_DIRECTORY_PREPARE,
                (RootComponentChildDirectoryPreparationRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("prepare project instance Directory transport");
        assert_eq!(
            prepared
                .expect("prepare project instance Directory")
                .child
                .phase,
            ComponentRuntimePhase::DirectoryPrepared
        );

        let activated: Result<RootComponentChildRuntimeActivationResponse, Error> = fixture
            .pic()
            .update_call_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_RUNTIME_ACTIVATE,
                (RootComponentChildRuntimeActivationRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("activate project instance runtime transport");
        assert_eq!(
            activated
                .expect("activate project instance runtime")
                .child
                .phase,
            ComponentRuntimePhase::Active
        );

        let membership: Result<RootComponentChildMembershipActivationResponse, Error> = fixture
            .pic()
            .update_call_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_MEMBERSHIP_ACTIVATE,
                (RootComponentChildMembershipActivationRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("activate project instance membership transport");
        let membership = membership.expect("activate project instance membership");
        assert_eq!(membership.registry.status, ComponentLifecycleStatus::Active);
        assert_eq!(membership.child.phase, ComponentRuntimePhase::Active);
        (child, membership.registry.head)
    }

    fn assert_root_canister_summary(fixture: &ActiveComponentRegistryFixture) {
        let summary: Result<FleetSubnetRootCanisterSummary, Error> = fixture
            .pic()
            .query_call(fixture.root, CANIC_FLEET_SUBNET_ROOT_CANISTER_SUMMARY, ())
            .expect("query Fleet Subnet Root Canister summary");
        let summary = summary.expect("Fleet Subnet Root Canister summary");
        let coordinator_version: Result<canic::dto::fleet_registry::FleetRegistryVersion, Error> =
            fixture
                .pic()
                .query_call(fixture.coordinator, CANIC_FLEET_REGISTRY_VERSION, ())
                .expect("query Coordinator Registry version");

        assert_eq!(
            summary.fleet_registry,
            coordinator_version.expect("Coordinator Registry version")
        );
        assert_eq!(summary.fleet_subnet_root, fixture.root);
        assert_eq!(summary.placement_subnet, fixture.issuer.placement_subnet);
        assert_eq!(summary.status, FleetSubnetRootStatus::Active);
        assert_eq!(summary.infrastructure_canisters, 2);
        assert_eq!(summary.component_canisters, 2);
        assert_eq!(summary.total_canisters, 4);
    }

    fn install_fixture_coordinator(
        pic: &Pic,
        coordinator: Principal,
        coordinator_wasm: Vec<u8>,
        fixture: &BootstrappedRootFixture,
    ) {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root");
        let config_path = root_canister_config_path(workspace_root);
        let config = AppConfigSnapshot::load(&config_path).expect("load root config");
        let coordinator_args = FleetCoordinatorInitArgs {
            configured_app: fixture
                .init_args
                .authority
                .binding
                .authority
                .binding
                .fleet
                .app
                .clone(),
            authority: fixture.init_args.authority.binding.authority.clone(),
            component_topology: config.component_topology().clone(),
        };
        pic.install_canister(
            coordinator,
            coordinator_wasm,
            encode_one(coordinator_args).expect("encode Coordinator init"),
            None,
        );
    }

    fn assert_registry_and_root_runtime_activation(
        pic: &Pic,
        coordinator: Principal,
        fixture: &BootstrappedRootFixture,
        joining_version: canic::dto::fleet_registry::FleetRegistryVersion,
        sync_request: FleetSubnetRootRegistrySyncRequest,
    ) -> ActiveComponentBindings {
        let activated: Result<FleetRegistryActivationResponse, Error> = pic
            .update_call(
                coordinator,
                CANIC_FLEET_REGISTRY_ACTIVATE,
                (FleetRegistryActivationRequest {
                    expected_registry: joining_version,
                },),
            )
            .expect("activate Registry transport");
        let activated = activated.expect("activate Registry");
        assert_eq!(activated.version.revision, 3);
        let active: Result<FleetRegistry, Error> = pic
            .query_call(coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query active Registry");
        let active = active.expect("active Registry");
        assert_eq!(
            active.fleet_subnet_roots.first().expect("one root").status,
            FleetSubnetRootStatus::Active
        );
        let directory = FleetDirectorySnapshot {
            provenance: FleetDirectoryProvenance {
                registry: activated.version.clone(),
                source_fleet_subnet_root: fixture.root_id,
            },
            fleet_subnet_roots: active
                .fleet_subnet_roots
                .iter()
                .map(|entry| FleetSubnetRootDirectoryEntry {
                    placement_subnet: entry.placement_subnet,
                    fleet_subnet_root: entry.fleet_subnet_root,
                    status: entry.status,
                })
                .collect(),
        };
        let activation_request = FleetSubnetRootRegistryMirrorActivationRequest {
            previous_registry: activated.previous_version,
            expected_registry: activated.version,
            expected_directory: directory,
            store_bootstrap: fixture.request.clone(),
        };
        let mirror: Result<FleetSubnetRootRegistryMirrorActivationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
                (activation_request.clone(),),
            )
            .expect("activate root Registry mirror transport");
        let mirror = mirror.expect("activate root Registry mirror");
        let mirror_retry: Result<FleetSubnetRootRegistryMirrorActivationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
                (activation_request.clone(),),
            )
            .expect("retry root Registry mirror activation transport");
        assert_eq!(
            mirror_retry.expect("retry root Registry mirror activation"),
            mirror
        );
        let mirror_status: Result<FleetSubnetRootRegistryMirrorActivationResponse, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_MIRROR_STATUS,
                (activation_request.clone(),),
            )
            .expect("query root Registry mirror status transport");
        assert_eq!(mirror_status.expect("root Registry mirror status"), mirror);

        let issuer = assert_component_registry_preparation(pic, fixture, activation_request);

        let old_candidate: Result<FleetSubnetRootRegistrySyncResponse, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_SYNC_STATUS,
                (sync_request,),
            )
            .expect("query private Joining candidate after Registry activation");
        assert_eq!(
            old_candidate
                .expect_err("Joining candidate must be replaced")
                .code,
            canic::dto::error::ErrorCode::Unavailable
        );
        issuer
    }

    fn assert_component_registry_preparation(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        activation_request: FleetSubnetRootRegistryMirrorActivationRequest,
    ) -> ActiveComponentBindings {
        let component_registry_request = RootComponentRegistryPreparationRequest {
            store_bootstrap: activation_request.store_bootstrap,
            expected_fleet_registry: activation_request.expected_registry,
        };
        let component_registry: Result<RootComponentRegistryStatusResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_PREPARE,
                (component_registry_request.clone(),),
            )
            .expect("prepare root Component Registry transport");
        let component_registry = component_registry.expect("prepare root Component Registry");
        assert_eq!(component_registry.fleet_subnet_root, fixture.root_id);
        assert_eq!(
            component_registry.release_set,
            fixture.init_args.authority.initial_release_set
        );
        assert_eq!(
            component_registry.component_topology_digest,
            fixture
                .init_args
                .authority
                .binding
                .component_topology_digest
        );
        assert_eq!(component_registry.next_allocation_sequence, 1);
        assert_eq!(component_registry.reserved_component_instances, 0);
        assert_eq!(component_registry.committed_component_instances, 0);
        assert_eq!(component_registry.managed_descendants, 0);
        assert_eq!(component_registry.known_created_component_canisters, 0);
        assert_eq!(component_registry.encoded_bytes, 0);

        let component_registry_retry: Result<RootComponentRegistryStatusResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_PREPARE,
                (component_registry_request.clone(),),
            )
            .expect("retry root Component Registry preparation transport");
        assert_eq!(
            component_registry_retry.expect("retry root Component Registry preparation"),
            component_registry
        );
        let component_registry_status: Result<RootComponentRegistryStatusResponse, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                (component_registry_request.clone(),),
            )
            .expect("query root Component Registry status transport");
        assert_eq!(
            component_registry_status.expect("root Component Registry status"),
            component_registry
        );

        assert_component_allocation(pic, fixture, component_registry_request)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the activation fixture verifies both initial Component allocations and their shared inventory transition"
    )]
    fn assert_component_allocation(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        component_registry_request: RootComponentRegistryPreparationRequest,
    ) -> ActiveComponentBindings {
        let (issuer_request, issuer) = assert_issuer_component_allocation(pic, fixture);
        let issuer_binding = installed_component_binding(&issuer);
        let projects_request = RootComponentAllocationRequest {
            operation_id: [0xa2; 32],
            component_spec: "projects".parse().expect("projects Component Spec"),
        };
        let projects: Result<RootComponentAllocationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (projects_request.clone(),),
            )
            .expect("reserve projects Component transport");
        let projects = projects.expect("reserve projects Component");
        assert_eq!(projects.allocation_sequence, 2);
        assert_ne!(projects.component, issuer.component);
        assert_eq!(projects.role, CanisterRole::new("project_hub"));
        assert_eq!(projects.phase, RootComponentAllocationPhase::Reserved);
        let projects_component = projects.component;

        let conflicting_retry: Result<RootComponentAllocationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (RootComponentAllocationRequest {
                    operation_id: issuer_request.operation_id,
                    component_spec: projects_request.component_spec.clone(),
                },),
            )
            .expect("conflicting Component reservation retry transport");
        assert_eq!(
            conflicting_retry
                .expect_err("conflicting Component reservation retry must fail")
                .code,
            canic::dto::error::ErrorCode::Conflict
        );

        let exhausted: Result<RootComponentAllocationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (RootComponentAllocationRequest {
                    operation_id: [0xa3; 32],
                    component_spec: issuer_request.component_spec,
                },),
            )
            .expect("exhausted issuer Component reservation transport");
        assert_eq!(
            exhausted
                .expect_err("issuer Component admission must be exhausted")
                .code,
            canic::dto::error::ErrorCode::ResourceExhausted
        );

        let component_registry: Result<RootComponentRegistryStatusResponse, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                (component_registry_request.clone(),),
            )
            .expect("query allocated root Component Registry status transport");
        let component_registry =
            component_registry.expect("allocated root Component Registry status");
        assert_eq!(component_registry.next_allocation_sequence, 3);
        assert_eq!(component_registry.reserved_component_instances, 1);
        assert_eq!(component_registry.committed_component_instances, 1);
        assert_eq!(component_registry.managed_descendants, 0);
        assert_eq!(component_registry.known_created_component_canisters, 1);
        assert!(component_registry.encoded_bytes > 0);
        assert_prepared(pic, fixture.root_id);
        let incomplete_activation: Result<FleetActivationStatusResponse, Error> = pic
            .update_call(fixture.root_id, CANIC_PREPARE_FLEET_ACTIVATION, ())
            .expect("prepare incomplete root Fleet activation transport");
        assert_eq!(
            incomplete_activation
                .expect_err("reserved Component must prevent root activation preparation")
                .code,
            canic::dto::error::ErrorCode::Unavailable
        );
        assert_eq!(
            component_registry_status(pic, fixture, component_registry_request.clone())
                .initial_inventory,
            None
        );

        let active_projects = create_component(pic, fixture, projects_request.operation_id);
        assert_eq!(active_projects.component, projects_component);
        assert_eq!(
            active_projects.component_spec,
            projects_request.component_spec
        );
        assert_eq!(active_projects.role, CanisterRole::new("project_hub"));
        assert_eq!(
            active_projects.phase,
            RootComponentAllocationPhase::Committed
        );
        let complete = component_registry_status(pic, fixture, component_registry_request.clone());
        assert_eq!(complete.reserved_component_instances, 0);
        assert_eq!(complete.committed_component_instances, 2);
        assert_eq!(complete.known_created_component_canisters, 2);
        assert_eq!(complete.initial_inventory, None);
        assert_root_runtime_activation(pic, fixture, component_registry_request);
        ActiveComponentBindings::new(
            issuer_binding,
            installed_component_binding(&active_projects),
        )
    }

    fn installed_component_binding(
        allocation: &RootComponentAllocationResponse,
    ) -> ComponentBinding {
        allocation
            .installation
            .as_ref()
            .expect("Component installation evidence")
            .binding
            .clone()
    }

    fn assert_root_runtime_activation(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        component_registry_request: RootComponentRegistryPreparationRequest,
    ) {
        let prepared: Result<FleetActivationStatusResponse, Error> = pic
            .update_call(fixture.root_id, CANIC_PREPARE_FLEET_ACTIVATION, ())
            .expect("prepare root Fleet activation transport");
        let prepared = prepared.expect("prepare root Fleet activation");
        assert_eq!(prepared.phase, FleetActivationPhase::Prepared);
        let credential = prepared
            .credential
            .expect("prepared root credential generation");

        let sealed = component_registry_status(pic, fixture, component_registry_request.clone())
            .initial_inventory
            .expect("sealed initial Component inventory");
        assert_initial_inventory(&sealed, prepared.identity.operation_id, 2, false, false);

        let request = FleetActivationResumeRequest {
            operation_id: prepared.identity.operation_id,
            credential,
        };
        let activated: Result<FleetActivationStatusResponse, Error> = pic
            .update_call(fixture.root_id, CANIC_RESUME_FLEET_ACTIVATION, (request,))
            .expect("resume root Fleet activation transport");
        let activated = activated.expect("resume root Fleet activation");
        assert_eq!(activated.phase, FleetActivationPhase::Active);
        assert!(activated.activated_at_ns.is_some_and(|time| time > 0));

        let terminal = component_registry_status(pic, fixture, component_registry_request)
            .initial_inventory
            .expect("terminal initial Component inventory");
        assert_initial_inventory(&terminal, request.operation_id, 2, true, true);
        assert_eq!(terminal.inventory_hash, sealed.inventory_hash);

        let retried: Result<FleetActivationStatusResponse, Error> = pic
            .update_call(fixture.root_id, CANIC_RESUME_FLEET_ACTIVATION, (request,))
            .expect("retry root Fleet activation transport");
        assert_eq!(
            retried.expect("retry root Fleet activation"),
            activated,
            "root activation retry must preserve its original receipt"
        );
    }

    fn component_registry_status(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        request: RootComponentRegistryPreparationRequest,
    ) -> RootComponentRegistryStatusResponse {
        let status: Result<RootComponentRegistryStatusResponse, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                (request,),
            )
            .expect("query root Component Registry activation status transport");
        status.expect("root Component Registry activation status")
    }

    fn assert_initial_inventory(
        inventory: &RootComponentInitialInventoryStatus,
        operation_id: [u8; 32],
        component_count: u32,
        directories_converged: bool,
        root_runtime_activated: bool,
    ) {
        assert_eq!(inventory.fleet_activation_operation_id, operation_id);
        assert_eq!(inventory.component_count, component_count);
        assert_ne!(inventory.inventory_hash, [0; 32]);
        assert!(inventory.sealed_at_ns > 0);
        assert_eq!(inventory.directories_converged, directories_converged);
        assert_eq!(inventory.root_runtime_activated, root_runtime_activated);
    }

    fn assert_issuer_component_allocation(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
    ) -> (
        RootComponentAllocationRequest,
        RootComponentAllocationResponse,
    ) {
        let issuer_request = RootComponentAllocationRequest {
            operation_id: [0xa1; 32],
            component_spec: "issuer".parse().expect("issuer Component Spec"),
        };
        let issuer: Result<RootComponentAllocationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (issuer_request.clone(),),
            )
            .expect("reserve issuer Component transport");
        let issuer = issuer.expect("reserve issuer Component");
        assert_eq!(issuer.operation_id, issuer_request.operation_id);
        assert_eq!(issuer.allocation_sequence, 1);
        assert_eq!(issuer.component_spec, issuer_request.component_spec);
        assert_eq!(issuer.role, CanisterRole::new("issuer"));
        assert_eq!(
            issuer.component,
            ComponentInstanceId::from_root_allocation(
                fixture
                    .init_args
                    .authority
                    .binding
                    .authority
                    .binding
                    .fleet
                    .fleet,
                fixture.init_args.authority.binding.authority.epoch,
                fixture.root_id,
                1,
            )
        );
        assert_eq!(
            issuer.provisioning_origin,
            ComponentProvisioningOrigin::FleetAdministrator {
                caller: Principal::anonymous(),
            }
        );
        assert_eq!(
            issuer.release_set,
            fixture.init_args.authority.initial_release_set
        );
        assert_eq!(issuer.phase, RootComponentAllocationPhase::Reserved);

        let issuer_retry: Result<RootComponentAllocationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (issuer_request.clone(),),
            )
            .expect("retry issuer Component reservation transport");
        assert_eq!(
            issuer_retry.expect("retry issuer Component reservation"),
            issuer
        );
        let issuer_status: Result<RootComponentAllocationResponse, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
                (RootComponentAllocationStatusRequest {
                    operation_id: issuer_request.operation_id,
                },),
            )
            .expect("query issuer Component reservation transport");
        assert_eq!(
            issuer_status.expect("issuer Component reservation status"),
            issuer
        );

        let created = create_component(pic, fixture, issuer_request.operation_id);
        (issuer_request, created)
    }

    fn create_component(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        operation_id: [u8; 32],
    ) -> RootComponentAllocationResponse {
        let created: Result<RootComponentAllocationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_CREATE,
                (RootComponentCreationRequest { operation_id },),
            )
            .expect("create Component transport");
        let created = created.expect("create Component");
        assert_eq!(created.phase, RootComponentAllocationPhase::Created);
        let creation = created.creation.as_ref().expect("creation evidence");
        let canister = creation.canister.expect("created Canister");
        assert_eq!(creation.wasm_store, fixture.response.wasm_store);
        assert_eq!(creation.controller, fixture.root_id);
        assert!(creation.initial_cycles.to_u128() > 0);
        let artifact = fixture
            .response
            .catalog
            .iter()
            .find(|entry| entry.role == created.role)
            .expect("Component Store artifact");
        assert_eq!(creation.payload_hash, artifact.payload_hash);
        assert_eq!(creation.payload_size_bytes, artifact.payload_size_bytes);

        let canister_status = pic
            .canister_status(canister, Some(fixture.root_id))
            .expect("created Component Canister status");
        assert_eq!(canister_status.settings.controllers, vec![fixture.root_id]);
        assert_eq!(canister_status.module_hash, None);
        assert!(
            canister_status.cycles > 0_u128
                && canister_status.cycles <= creation.initial_cycles.to_u128(),
            "the created Canister must retain cycles from the exact frozen creation funding"
        );

        let retry: Result<RootComponentAllocationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_CREATE,
                (RootComponentCreationRequest { operation_id },),
            )
            .expect("retry Component creation transport");
        assert_eq!(retry.expect("retry Component creation"), created);
        let created_status: Result<RootComponentAllocationResponse, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
                (RootComponentAllocationStatusRequest { operation_id },),
            )
            .expect("query created Component transport");
        assert_eq!(created_status.expect("created Component status"), created);

        install_component(pic, fixture, operation_id, created)
    }

    fn install_component(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        operation_id: [u8; 32],
        created: RootComponentAllocationResponse,
    ) -> RootComponentAllocationResponse {
        let creation = created.creation.as_ref().expect("creation evidence");
        let artifact = fixture
            .response
            .catalog
            .iter()
            .find(|entry| entry.role == created.role)
            .expect("Component Store artifact");
        let installed: Result<RootComponentAllocationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_INSTALL,
                (RootComponentInstallRequest { operation_id },),
            )
            .expect("install Component transport");
        let installed = installed.expect("install Component");
        assert_eq!(installed.phase, RootComponentAllocationPhase::Verified);
        let installation = installed.installation.as_ref().expect("install evidence");
        assert_eq!(installation.raw_module_hash, artifact.raw_module_hash);
        assert_eq!(
            installation.binding.canister_id,
            creation.canister.expect("created Canister")
        );
        assert_eq!(
            installation.binding.component, installed.component,
            "target binding must retain the reserved Component identity"
        );
        let observed_binding: Result<ManagedCanisterBinding, Error> = pic
            .query_call_as(
                creation.canister.expect("created Canister"),
                fixture.root_id,
                canic::protocol::CANIC_MANAGED_CANISTER_BINDING,
                (),
            )
            .expect("query installed Component binding transport");
        assert_eq!(
            observed_binding.expect("installed Component binding"),
            ManagedCanisterBinding::Component(installation.binding.clone())
        );
        let installed_status = pic
            .canister_status(
                creation.canister.expect("created Canister"),
                Some(fixture.root_id),
            )
            .expect("installed Component Canister status");
        assert_eq!(
            installed_status.module_hash,
            Some(creation.payload_hash.to_vec())
        );

        let install_retry: Result<RootComponentAllocationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_INSTALL,
                (RootComponentInstallRequest { operation_id },),
            )
            .expect("retry Component install transport");
        assert_eq!(install_retry.expect("retry Component install"), installed);
        commit_component(pic, fixture, operation_id, installed)
    }

    fn commit_component(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        operation_id: [u8; 32],
        installed: RootComponentAllocationResponse,
    ) -> RootComponentAllocationResponse {
        let committed: Result<RootComponentCommitResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_COMMIT,
                (RootComponentCommitRequest { operation_id },),
            )
            .expect("commit Component transport");
        let committed = committed.expect("commit Component");
        assert_eq!(
            committed.allocation.phase,
            RootComponentAllocationPhase::Committed
        );
        assert_eq!(committed.allocation.component, installed.component);
        assert_eq!(
            committed.allocation.installation, installed.installation,
            "Registry commitment must retain the verified install evidence"
        );
        assert_eq!(committed.registry.head.component, installed.component);
        assert_eq!(committed.registry.head.revision, 1);
        assert_ne!(committed.registry.head.content_hash, [0; 32]);
        assert_eq!(
            committed.registry.binding,
            installed
                .installation
                .as_ref()
                .expect("verified installation")
                .binding
        );
        assert_eq!(
            committed.registry.provisioning_origin,
            installed.provisioning_origin
        );
        assert_eq!(committed.registry.release_set, installed.release_set);
        assert_eq!(
            committed.registry.status,
            ComponentLifecycleStatus::Prepared
        );
        assert!(committed.registry.encoded_bytes > 0);
        assert_eq!(
            committed.directory.provenance.component,
            committed.registry.binding
        );
        assert_eq!(
            committed.directory.provenance.source_fleet_subnet_root,
            fixture.root_id
        );
        assert_eq!(
            committed.directory.provenance.component_registry_revision,
            committed.registry.head.revision
        );
        assert_eq!(
            committed
                .directory
                .provenance
                .component_registry_content_hash,
            committed.registry.head.content_hash
        );
        assert!(
            committed.directory.provenance.synchronized_at_ns > 0,
            "the first Directory must retain its derivation time"
        );
        assert_eq!(committed.directory.descendant_count, 0);

        let retry: Result<RootComponentCommitResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_COMMIT,
                (RootComponentCommitRequest { operation_id },),
            )
            .expect("retry Component commitment transport");
        assert_eq!(retry.expect("retry Component commitment"), committed);
        assert_committed_component_queries(pic, fixture, operation_id, &committed);
        prepare_component_directories(pic, fixture, operation_id, committed)
    }

    fn prepare_component_directories(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        operation_id: [u8; 32],
        committed: RootComponentCommitResponse,
    ) -> RootComponentAllocationResponse {
        let target = committed.registry.binding.canister_id;
        let awaiting: Result<ComponentRuntimeStatusResponse, Error> = pic
            .query_call_as(target, fixture.root_id, CANIC_COMPONENT_RUNTIME_STATUS, ())
            .expect("query awaiting Component runtime Directory transport");
        let awaiting = awaiting.expect("awaiting Component runtime Directory");
        assert_eq!(awaiting.operation_id, operation_id);
        assert_eq!(
            awaiting.binding,
            ManagedCanisterBinding::Component(committed.registry.binding.clone())
        );
        assert_eq!(awaiting.phase, ComponentRuntimePhase::AwaitingDirectory);
        assert_eq!(awaiting.authority, None);
        assert_eq!(awaiting.authority_hash, None);
        assert_eq!(awaiting.activation, None);

        let request = RootComponentDirectoryPreparationRequest { operation_id };
        let prepared: Result<RootComponentDirectoryPreparationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_DIRECTORY_PREPARE,
                (request,),
            )
            .expect("prepare Component Directories transport");
        let prepared = prepared.expect("prepare Component Directories");
        assert_eq!(prepared.committed, committed);
        assert_eq!(
            prepared.target.phase,
            ComponentRuntimePhase::DirectoryPrepared
        );
        assert_eq!(prepared.target.operation_id, operation_id);
        assert_eq!(
            prepared.target.binding,
            ManagedCanisterBinding::Component(prepared.committed.registry.binding.clone())
        );
        let authority = prepared
            .target
            .authority
            .as_ref()
            .expect("retained Component runtime Directory authority");
        assert_eq!(authority.component, prepared.committed.directory);
        assert_eq!(
            authority.fleet.provenance.source_fleet_subnet_root,
            fixture.root_id
        );
        assert!(
            authority
                .fleet
                .fleet_subnet_roots
                .iter()
                .all(|entry| entry.status == FleetSubnetRootStatus::Active)
        );
        assert_ne!(
            prepared
                .target
                .authority_hash
                .expect("Directory authority hash"),
            [0; 32]
        );
        assert_eq!(prepared.target.activation, None);

        let retry: Result<RootComponentDirectoryPreparationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_DIRECTORY_PREPARE,
                (request,),
            )
            .expect("retry Component Directory preparation transport");
        assert_eq!(
            retry.expect("retry Component Directory preparation"),
            prepared
        );
        let observed: Result<ComponentRuntimeStatusResponse, Error> = pic
            .query_call_as(target, fixture.root_id, CANIC_COMPONENT_RUNTIME_STATUS, ())
            .expect("query prepared Component runtime Directory transport");
        assert_eq!(
            observed.expect("prepared Component runtime Directory"),
            prepared.target
        );
        assert_prepared(pic, fixture.root_id);
        let target_activation: Result<FleetActivationStatusResponse, Error> = pic
            .query_call_as(target, fixture.root_id, CANIC_FLEET_ACTIVATION_STATUS, ())
            .expect("query prepared Component activation transport");
        assert_eq!(
            target_activation
                .expect("prepared Component activation")
                .phase,
            FleetActivationPhase::Prepared
        );
        activate_component_runtime(pic, fixture, request, prepared)
    }

    fn activate_component_runtime(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        directory_request: RootComponentDirectoryPreparationRequest,
        prepared: RootComponentDirectoryPreparationResponse,
    ) -> RootComponentAllocationResponse {
        let target = prepared.committed.registry.binding.canister_id;
        let request = RootComponentRuntimeActivationRequest {
            operation_id: directory_request.operation_id,
        };
        let activated: Result<RootComponentRuntimeActivationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_RUNTIME_ACTIVATE,
                (request,),
            )
            .expect("activate Component runtime transport");
        let activated = activated.expect("activate Component runtime");
        assert_eq!(activated.committed, prepared.committed);
        assert_eq!(activated.target.operation_id, request.operation_id);
        assert_eq!(activated.target.binding, prepared.target.binding);
        assert_eq!(activated.target.phase, ComponentRuntimePhase::Active);
        assert_eq!(activated.target.authority, prepared.target.authority);
        assert_eq!(
            activated.target.authority_hash,
            prepared.target.authority_hash
        );
        let activation = activated
            .target
            .activation
            .as_ref()
            .expect("Component runtime activation evidence");
        assert_eq!(
            Some(activation.directory_authority_hash),
            activated.target.authority_hash
        );
        assert!(activation.activated_at_ns > 0);

        let retry: Result<RootComponentRuntimeActivationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_RUNTIME_ACTIVATE,
                (request,),
            )
            .expect("retry Component runtime activation transport");
        assert_eq!(
            retry.expect("retry Component runtime activation"),
            activated
        );
        let observed: Result<ComponentRuntimeStatusResponse, Error> = pic
            .query_call_as(target, fixture.root_id, CANIC_COMPONENT_RUNTIME_STATUS, ())
            .expect("query active Component runtime transport");
        assert_eq!(
            observed.expect("active Component runtime"),
            activated.target
        );
        let target_activation: Result<FleetActivationStatusResponse, Error> = pic
            .query_call_as(target, fixture.root_id, CANIC_FLEET_ACTIVATION_STATUS, ())
            .expect("query active Component Fleet status transport");
        let target_activation = target_activation.expect("active Component Fleet status");
        assert_eq!(target_activation.phase, FleetActivationPhase::Active);
        assert_eq!(target_activation.cascade, None);
        assert_eq!(target_activation.credential, None);
        assert_eq!(
            target_activation.activated_at_ns,
            Some(activation.activated_at_ns)
        );

        let prepared_retry: Result<RootComponentDirectoryPreparationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_DIRECTORY_PREPARE,
                (directory_request,),
            )
            .expect("retry Directory preparation after runtime activation transport");
        assert_eq!(
            prepared_retry.expect("retry Directory preparation after runtime activation"),
            prepared
        );
        assert_committed_component_queries(
            pic,
            fixture,
            request.operation_id,
            &activated.committed,
        );
        assert_prepared(pic, fixture.root_id);
        activate_component_membership(pic, fixture, directory_request, prepared, activated)
    }

    fn activate_component_membership(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        directory_request: RootComponentDirectoryPreparationRequest,
        prepared: RootComponentDirectoryPreparationResponse,
        activated: RootComponentRuntimeActivationResponse,
    ) -> RootComponentAllocationResponse {
        let request = RootComponentMembershipActivationRequest {
            operation_id: directory_request.operation_id,
        };
        let membership: Result<RootComponentMembershipActivationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_MEMBERSHIP_ACTIVATE,
                (request,),
            )
            .expect("activate Component membership transport");
        let membership = membership.expect("activate Component membership");
        assert_eq!(membership.allocation, activated.committed.allocation);
        assert_eq!(membership.registry.status, ComponentLifecycleStatus::Active);
        assert_eq!(membership.registry.head.revision, 2);
        assert_ne!(
            membership.registry.head.content_hash,
            activated.committed.registry.head.content_hash
        );
        assert_eq!(
            membership.directory.provenance.component_registry_revision,
            membership.registry.head.revision
        );
        assert_eq!(
            membership
                .directory
                .provenance
                .component_registry_content_hash,
            membership.registry.head.content_hash
        );
        assert!(
            membership.directory.provenance.synchronized_at_ns
                > activated.committed.directory.provenance.synchronized_at_ns
        );
        assert_eq!(membership.target.phase, ComponentRuntimePhase::Active);
        assert_eq!(
            membership
                .target
                .authority
                .as_ref()
                .expect("current active Directory")
                .component,
            membership.directory
        );
        assert_eq!(
            membership
                .target
                .activation
                .expect("immutable activation receipt")
                .directory_authority_hash,
            activated
                .target
                .authority_hash
                .expect("prepared activation authority hash")
        );

        assert_active_membership_queries(pic, fixture, &membership);
        let retry: Result<RootComponentMembershipActivationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_MEMBERSHIP_ACTIVATE,
                (request,),
            )
            .expect("retry Component membership transport");
        assert_eq!(retry.expect("retry Component membership"), membership);

        assert_pre_membership_retries(pic, fixture, directory_request, prepared, activated);
        assert_prepared(pic, fixture.root_id);
        membership.allocation
    }

    fn assert_active_membership_queries(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        membership: &RootComponentMembershipActivationResponse,
    ) {
        let registry: Result<ComponentRegistryPartitionResponse, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
                (ComponentRegistryPartitionRequest {
                    component: membership.allocation.component,
                },),
            )
            .expect("query active Component Registry partition transport");
        assert_eq!(
            registry.expect("active Component Registry partition"),
            membership.registry
        );
        let directory: Result<ComponentDirectoryHead, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_DIRECTORY_HEAD,
                (ComponentDirectoryHeadRequest {
                    component: membership.allocation.component,
                },),
            )
            .expect("query active Component Directory transport");
        assert_eq!(
            directory.expect("active Component Directory"),
            membership.directory
        );
        let target: Result<ComponentRuntimeStatusResponse, Error> = pic
            .query_call_as(
                membership.registry.binding.canister_id,
                fixture.root_id,
                CANIC_COMPONENT_RUNTIME_STATUS,
                (),
            )
            .expect("query membership-active Component runtime transport");
        assert_eq!(
            target.expect("membership-active Component runtime"),
            membership.target
        );
    }

    fn assert_pre_membership_retries(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        directory_request: RootComponentDirectoryPreparationRequest,
        prepared: RootComponentDirectoryPreparationResponse,
        activated: RootComponentRuntimeActivationResponse,
    ) {
        let commit: Result<RootComponentCommitResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_COMMIT,
                (RootComponentCommitRequest {
                    operation_id: directory_request.operation_id,
                },),
            )
            .expect("retry Component commitment after membership activation transport");
        assert_eq!(
            commit.expect("retry Component commitment after membership activation"),
            activated.committed
        );
        let prepared_retry: Result<RootComponentDirectoryPreparationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_DIRECTORY_PREPARE,
                (directory_request,),
            )
            .expect("retry Directory preparation after membership activation transport");
        assert_eq!(
            prepared_retry.expect("retry Directory preparation after membership activation"),
            prepared
        );
        let activated_retry: Result<RootComponentRuntimeActivationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_RUNTIME_ACTIVATE,
                (RootComponentRuntimeActivationRequest {
                    operation_id: directory_request.operation_id,
                },),
            )
            .expect("retry runtime activation after membership activation transport");
        assert_eq!(
            activated_retry.expect("retry runtime activation after membership activation"),
            activated
        );
    }

    fn assert_committed_component_queries(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        operation_id: [u8; 32],
        committed: &RootComponentCommitResponse,
    ) {
        let allocation_status: Result<RootComponentAllocationResponse, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
                (RootComponentAllocationStatusRequest { operation_id },),
            )
            .expect("query committed Component transport");
        assert_eq!(
            allocation_status.expect("committed Component status"),
            committed.allocation
        );

        let registry: Result<ComponentRegistryPartitionResponse, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
                (ComponentRegistryPartitionRequest {
                    component: committed.allocation.component,
                },),
            )
            .expect("query Component Registry partition transport");
        assert_eq!(
            registry.expect("Component Registry partition"),
            committed.registry
        );

        let directory: Result<ComponentDirectoryHead, Error> = pic
            .query_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_DIRECTORY_HEAD,
                (ComponentDirectoryHeadRequest {
                    component: committed.allocation.component,
                },),
            )
            .expect("query Component Directory head transport");
        assert_eq!(
            directory.expect("Component Directory head"),
            committed.directory
        );
    }

    fn install_bootstrapped_root(
        pic: &Pic,
        root_wasm: Vec<u8>,
        coordinator: Principal,
        store_fixture: RootStoreFixture,
    ) -> BootstrappedRootFixture {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root");
        let config_path = root_canister_config_path(workspace_root);
        let RootStoreFixture {
            manifest,
            artifacts,
        } = store_fixture;
        let manifest_bytes = serde_json::to_vec(&manifest).expect("canonical root release set");
        let digest = ReleaseSetDigest::from_bytes(
            wasm_hash(&manifest_bytes)
                .try_into()
                .expect("SHA-256 digest"),
        );
        let root_id = pic.create_canister();
        pic.add_cycles(root_id, ROOT_INSTALL_CYCLES);
        let init_bytes = install_root_args_with_release_set_digest_and_coordinator(
            root_id,
            coordinator,
            &root_wasm,
            &config_path,
            digest,
        )
        .expect("encode exact root authority");
        let init_args =
            decode_one::<FleetSubnetRootInitArgs>(&init_bytes).expect("decode root init authority");
        pic.install_canister(root_id, root_wasm, init_bytes, None);
        assert_prepared(pic, root_id);

        let version = TemplateVersion::owned(manifest.release_build_id.to_string());
        stage_chunked_payload(
            pic,
            root_id,
            TemplateId::owned(format!("{ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX}{digest}")),
            version.clone(),
            &manifest_bytes,
        );
        for (role, bytes) in artifacts {
            let template_id =
                TemplateId::owned(format!("{ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX}{role}"));
            let staged: Result<(), Error> = pic
                .update_call(
                    root_id,
                    CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
                    (TemplateManifestInput {
                        template_id: template_id.clone(),
                        role,
                        version: version.clone(),
                        payload_hash: wasm_hash(&bytes),
                        payload_size_bytes: bytes.len() as u64,
                        store_binding: WasmStoreBinding::new("bootstrap"),
                        chunking_mode: TemplateChunkingMode::Chunked,
                        manifest_state: TemplateManifestState::Approved,
                        approved_at: Some(0),
                        created_at: 0,
                    },),
                )
                .expect("stage artifact manifest transport");
            staged.expect("stage artifact manifest");
            stage_chunked_payload(pic, root_id, template_id, version.clone(), &bytes);
        }

        let request = RootStoreBootstrapRequest {
            manifest_payload_size_bytes: manifest_bytes.len() as u64,
        };
        let response: Result<RootStoreBootstrapResponse, Error> = pic
            .update_call(root_id, CANIC_ROOT_STORE_BOOTSTRAP, (request.clone(),))
            .expect("root Store bootstrap transport");
        BootstrappedRootFixture {
            root_id,
            init_args,
            request,
            response: response.expect("root Store bootstrap"),
        }
    }

    fn build_test_coordinator_wasm() -> Vec<u8> {
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let target_dir = test_target_dir(&workspace_root, "fleet-registry-sync");
        build_internal_test_wasm_canisters(
            &workspace_root,
            &target_dir,
            &[COORDINATOR_PACKAGE],
            CanicWasmBuildProfile::Fast,
        );
        read_wasm(
            &target_dir,
            COORDINATOR_PACKAGE,
            CanicWasmBuildProfile::Fast.target_dir_name(),
        )
    }

    fn build_root_store_fixture() -> RootStoreFixture {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root");
        let config_path = root_canister_config_path(workspace_root);
        let (manifest, artifacts) = exact_root_store_fixture(&config_path);
        RootStoreFixture {
            manifest,
            artifacts,
        }
    }

    fn exact_root_store_fixture(
        config_path: &Path,
    ) -> (RootStoreReleaseSetManifest, BTreeMap<CanisterRole, Vec<u8>>) {
        let config = AppConfigSnapshot::load(config_path).expect("load root fixture config");
        let topology = config.component_topology();
        let release_build_id = managed_test_init_identity().release_build_id;
        let mut entries = Vec::new();
        let mut artifacts = BTreeMap::new();
        let real_modules = BTreeMap::from([
            (CanisterRole::new("issuer"), build_test_issuer_wasm()),
            (
                CanisterRole::new("project_hub"),
                build_test_project_hub_wasm(),
            ),
            (
                CanisterRole::new("project_instance"),
                build_test_project_instance_wasm(),
            ),
        ]);
        for spec in &topology.component_specs {
            entries.push(root_store_entry(
                config.model(),
                &spec.component_spec,
                RootStoreReleaseSetEntryKind::Component,
                &spec.component_role,
                release_build_id,
                &real_modules,
                &mut artifacts,
            ));
            entries.extend(spec.children.iter().map(|child| {
                root_store_entry(
                    config.model(),
                    &spec.component_spec,
                    RootStoreReleaseSetEntryKind::ComponentChild,
                    &child.role,
                    release_build_id,
                    &real_modules,
                    &mut artifacts,
                )
            }));
        }

        (
            RootStoreReleaseSetManifest {
                release_build_id,
                component_topology_digest: topology.digest().expect("fixture topology digest"),
                entries,
            },
            artifacts,
        )
    }

    fn root_store_entry(
        config: &canic_core::bootstrap::compiled::ConfigModel,
        component_spec: &canic_core::ids::ComponentSpecId,
        kind: RootStoreReleaseSetEntryKind,
        role: &CanisterRole,
        release_build_id: canic_core::ids::ReleaseBuildId,
        real_modules: &BTreeMap<CanisterRole, Vec<u8>>,
        artifacts: &mut BTreeMap<CanisterRole, Vec<u8>>,
    ) -> RootStoreReleaseSetEntry {
        let raw = real_modules
            .get(role)
            .cloned()
            .unwrap_or_else(|| format!("raw fixture for {role}").into_bytes());
        let compressed = gzip(&raw);
        let existing = artifacts.insert(role.clone(), compressed.clone());
        assert!(
            existing.as_ref().is_none_or(|bytes| bytes == &compressed),
            "one role must retain one exact artifact payload"
        );
        RootStoreReleaseSetEntry {
            component_spec: component_spec.clone(),
            kind,
            artifact: RootStoreArtifact {
                role: role.clone(),
                package: config
                    .roles
                    .get(role)
                    .expect("fixture role declaration")
                    .package
                    .clone(),
                release_build_id,
                wasm_relative_path: format!("{role}.wasm"),
                wasm_size_bytes: raw.len() as u64,
                wasm_sha256_hex: hex_bytes(wasm_hash(&raw)),
                wasm_gz_relative_path: format!("{role}.wasm.gz"),
                wasm_gz_size_bytes: compressed.len() as u64,
                wasm_gz_sha256_hex: hex_bytes(wasm_hash(&compressed)),
            },
        }
    }

    fn build_test_issuer_wasm() -> Vec<u8> {
        static WASM: OnceLock<Vec<u8>> = OnceLock::new();
        WASM.get_or_init(|| {
            let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
            let target_dir = test_target_dir(&workspace_root, "fleet-registry-sync");
            let config_path = root_canister_config_path(&workspace_root);
            let canonical_config_path = config_path.to_str().expect("root config path UTF-8");
            build_internal_test_wasm_canisters_with_env(
                &workspace_root,
                &target_dir,
                &[ISSUER_PACKAGE],
                CanicWasmBuildProfile::Fast,
                &[(
                    canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
                    canonical_config_path,
                )],
            );
            read_wasm(
                &target_dir,
                ISSUER_PACKAGE,
                CanicWasmBuildProfile::Fast.target_dir_name(),
            )
        })
        .clone()
    }

    fn build_test_project_hub_wasm() -> Vec<u8> {
        static WASM: OnceLock<Vec<u8>> = OnceLock::new();
        WASM.get_or_init(|| {
            let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
            let target_dir = test_target_dir(&workspace_root, "fleet-registry-sync");
            let config_path = root_canister_config_path(&workspace_root);
            let canonical_config_path = config_path.to_str().expect("root config path UTF-8");
            build_internal_test_wasm_canisters_with_env(
                &workspace_root,
                &target_dir,
                &[PROJECT_HUB_PACKAGE],
                CanicWasmBuildProfile::Fast,
                &[(
                    canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
                    canonical_config_path,
                )],
            );
            read_wasm(
                &target_dir,
                PROJECT_HUB_PACKAGE,
                CanicWasmBuildProfile::Fast.target_dir_name(),
            )
        })
        .clone()
    }

    fn build_test_project_instance_wasm() -> Vec<u8> {
        static WASM: OnceLock<Vec<u8>> = OnceLock::new();
        WASM.get_or_init(|| {
            let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
            let target_dir = test_target_dir(&workspace_root, "fleet-registry-sync");
            let config_path = root_canister_config_path(&workspace_root);
            let canonical_config_path = config_path.to_str().expect("root config path UTF-8");
            build_internal_test_wasm_canisters_with_env(
                &workspace_root,
                &target_dir,
                &[PROJECT_INSTANCE_PACKAGE],
                CanicWasmBuildProfile::Fast,
                &[(
                    canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
                    canonical_config_path,
                )],
            );
            read_wasm(
                &target_dir,
                PROJECT_INSTANCE_PACKAGE,
                CanicWasmBuildProfile::Fast.target_dir_name(),
            )
        })
        .clone()
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(bytes).expect("gzip fixture Wasm");
        encoder.finish().expect("finish fixture Wasm gzip")
    }

    fn stage_chunked_payload(
        pic: &Pic,
        root_id: Principal,
        template_id: TemplateId,
        version: TemplateVersion,
        payload: &[u8],
    ) {
        let chunks = payload
            .chunks(CANIC_WASM_CHUNK_BYTES)
            .map(<[u8]>::to_vec)
            .collect::<Vec<_>>();
        let prepared: Result<TemplateChunkSetInfoResponse, Error> = pic
            .update_call(
                root_id,
                CANIC_TEMPLATE_PREPARE_ADMIN,
                (TemplateChunkSetPrepareInput {
                    template_id: template_id.clone(),
                    version: version.clone(),
                    payload_hash: wasm_hash(payload),
                    payload_size_bytes: payload.len() as u64,
                    chunk_hashes: chunks.iter().map(|chunk| wasm_hash(chunk)).collect(),
                },),
            )
            .expect("prepare staged payload transport");
        prepared.expect("prepare staged payload");
        for (chunk_index, bytes) in chunks.into_iter().enumerate() {
            let published: Result<(), Error> = pic
                .update_call(
                    root_id,
                    CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
                    (TemplateChunkInput {
                        template_id: template_id.clone(),
                        version: version.clone(),
                        chunk_index: u32::try_from(chunk_index).expect("bounded chunk index"),
                        bytes,
                    },),
                )
                .expect("publish staged payload transport");
            published.expect("publish staged payload");
        }
    }

    fn assert_prepared(pic: &Pic, root_id: Principal) {
        let status: Result<FleetActivationStatusResponse, Error> = pic
            .query_call(root_id, CANIC_FLEET_ACTIVATION_STATUS, ())
            .expect("query root activation status");
        assert_eq!(
            status.expect("root activation status").phase,
            FleetActivationPhase::Prepared
        );
        let authority: Result<FleetSubnetRootAuthority, Error> = pic
            .query_call(root_id, CANIC_FLEET_SUBNET_ROOT_AUTHORITY, ())
            .expect("query root authority");
        assert_eq!(
            authority.expect("root authority").binding.fleet_subnet_root,
            root_id
        );
    }
}

pub use tests::{ActiveComponentRegistryFixture, setup_active_component_registry};
