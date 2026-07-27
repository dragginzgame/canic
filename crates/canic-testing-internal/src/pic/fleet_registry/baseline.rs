//! Prepared-root Fleet Registry and Component Registry PocketIC journey.

use candid::Principal;
use ic_testkit::pic::Pic;
use std::path::Path;

use super::build::{build_pic, build_test_root_wasm, root_canister_config_path};

const ROOT_INSTALL_CYCLES: u128 = 80_000_000_000_000;

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{decode_one, encode_one};
    use canic::{
        CANIC_WASM_CHUNK_BYTES,
        dto::{
            component_registry::{
                ComponentProvisioningOrigin, RootComponentAllocationPhase,
                RootComponentAllocationRequest, RootComponentAllocationResponse,
                RootComponentAllocationStatusRequest, RootComponentRegistryPreparationRequest,
                RootComponentRegistryStatusResponse,
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
            fleet_subnet_root::{FleetSubnetRootAuthority, FleetSubnetRootInitArgs},
            root_store::{
                ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX, ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX,
                RootStoreArtifact, RootStoreBootstrapRequest, RootStoreBootstrapResponse,
                RootStoreReleaseSetEntry, RootStoreReleaseSetEntryKind,
                RootStoreReleaseSetManifest,
            },
        },
        ids::{CanisterRole, ComponentInstanceId, ReleaseSetDigest},
    };
    use canic::{
        Error,
        dto::fleet_activation::{FleetActivationPhase, FleetActivationStatusResponse},
        protocol::{
            CANIC_FLEET_ACTIVATION_STATUS, CANIC_FLEET_REGISTRY, CANIC_FLEET_REGISTRY_ACTIVATE,
            CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR, CANIC_FLEET_REGISTRY_MIRROR_STATUS,
            CANIC_FLEET_REGISTRY_ROOT_ACKNOWLEDGEMENTS, CANIC_FLEET_REGISTRY_SYNC_STATUS,
            CANIC_FLEET_REGISTRY_SYNCHRONIZE, CANIC_FLEET_REGISTRY_VERSION,
            CANIC_FLEET_SUBNET_ROOT_AUTHORITY, CANIC_FLEET_SUBNET_ROOT_JOIN,
            CANIC_ROOT_COMPONENT_ALLOCATE, CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
            CANIC_ROOT_COMPONENT_REGISTRY_PREPARE, CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
            CANIC_ROOT_STORE_BOOTSTRAP, CANIC_ROOT_STORE_BOOTSTRAP_STATUS,
            CANIC_TEMPLATE_PREPARE_ADMIN, CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
            CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN, CANIC_WASM_STORE_PREPARE,
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
    use std::collections::BTreeMap;

    use crate::pic::{
        CanicWasmBuildProfile, build_internal_test_wasm_canisters,
        canic::{
            install_root_args_with_release_set_digest_and_coordinator, managed_test_init_identity,
        },
    };
    use ic_testkit::artifacts::{read_wasm, test_target_dir, workspace_root_for};

    const COORDINATOR_PACKAGE: &str = "fleet_coordinator_stub";
    const COORDINATOR_INSTALL_CYCLES: u128 = 500_000_000_000_000;

    struct BootstrappedRootFixture {
        root_id: Principal,
        init_args: FleetSubnetRootInitArgs,
        request: RootStoreBootstrapRequest,
        response: RootStoreBootstrapResponse,
    }

    #[test]
    fn prepared_root_bootstraps_and_reverifies_its_exact_local_store() {
        let root_wasm = build_test_root_wasm();
        let pic = build_pic();
        let fixture =
            install_bootstrapped_root(&pic, root_wasm, Principal::from_slice(&[0x41; 29]));

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
    fn prepared_root_stages_and_acknowledges_the_exact_joining_registry_snapshot() {
        let root_wasm = build_test_root_wasm();
        let coordinator_wasm = build_test_coordinator_wasm();
        let pic = build_pic();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let fixture = install_bootstrapped_root(&pic, root_wasm, coordinator);
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

        assert_registry_activation_keeps_root_prepared(
            &pic,
            coordinator,
            &fixture,
            joined.version,
            sync_request,
        );
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

    fn assert_registry_activation_keeps_root_prepared(
        pic: &Pic,
        coordinator: Principal,
        fixture: &BootstrappedRootFixture,
        joining_version: canic::dto::fleet_registry::FleetRegistryVersion,
        sync_request: FleetSubnetRootRegistrySyncRequest,
    ) {
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

        assert_component_registry_preparation(pic, fixture, activation_request);

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
        assert_prepared(pic, fixture.root_id);
    }

    fn assert_component_registry_preparation(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        activation_request: FleetSubnetRootRegistryMirrorActivationRequest,
    ) {
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

        assert_component_allocation(pic, fixture, component_registry_request);
    }

    fn assert_component_allocation(
        pic: &Pic,
        fixture: &BootstrappedRootFixture,
        component_registry_request: RootComponentRegistryPreparationRequest,
    ) {
        let (issuer_request, issuer) = assert_issuer_component_allocation(pic, fixture);
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

        let conflicting_retry: Result<RootComponentAllocationResponse, Error> = pic
            .update_call(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (RootComponentAllocationRequest {
                    operation_id: issuer_request.operation_id,
                    component_spec: projects_request.component_spec,
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
                (component_registry_request,),
            )
            .expect("query allocated root Component Registry status transport");
        let component_registry =
            component_registry.expect("allocated root Component Registry status");
        assert_eq!(component_registry.next_allocation_sequence, 3);
        assert_eq!(component_registry.reserved_component_instances, 2);
        assert_eq!(component_registry.committed_component_instances, 0);
        assert_eq!(component_registry.managed_descendants, 0);
        assert!(component_registry.encoded_bytes > 0);
        assert_prepared(pic, fixture.root_id);
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
        (issuer_request, issuer)
    }

    fn install_bootstrapped_root(
        pic: &Pic,
        root_wasm: Vec<u8>,
        coordinator: Principal,
    ) -> BootstrappedRootFixture {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root");
        let config_path = root_canister_config_path(workspace_root);
        let (manifest, artifacts) = exact_root_store_fixture(&config_path);
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

    fn exact_root_store_fixture(
        config_path: &Path,
    ) -> (RootStoreReleaseSetManifest, BTreeMap<CanisterRole, Vec<u8>>) {
        let config = AppConfigSnapshot::load(config_path).expect("load root fixture config");
        let topology = config.component_topology();
        let release_build_id = managed_test_init_identity().release_build_id;
        let mut entries = Vec::new();
        let mut artifacts = BTreeMap::new();
        for spec in &topology.component_specs {
            entries.push(root_store_entry(
                config.model(),
                &spec.component_spec,
                RootStoreReleaseSetEntryKind::Component,
                &spec.component_role,
                release_build_id,
                &mut artifacts,
            ));
            entries.extend(spec.children.iter().map(|child| {
                root_store_entry(
                    config.model(),
                    &spec.component_spec,
                    RootStoreReleaseSetEntryKind::ComponentChild,
                    &child.role,
                    release_build_id,
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
        artifacts: &mut BTreeMap<CanisterRole, Vec<u8>>,
    ) -> RootStoreReleaseSetEntry {
        let raw = format!("raw fixture for {role}").into_bytes();
        let compressed = format!("compressed fixture for {role}").into_bytes();
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
