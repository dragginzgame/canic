//! Module: application_initialization::blob
//!
//! Responsibility: qualify the actual Blob artifact through Root's durable binding.
//! Boundary: use production build/manifest owners; no provider or paid effects.

use super::*;
use canic_blob_service::dto::configuration::{
    HostConfigurationView, HostFailure, ServiceBillingInput, ServiceConfigurationInput,
    ServiceFundingInput, ServiceInstallationInput, ServiceReadInput, ServiceResourceInput,
};
use canic_host::release_set::{
    CanicInfrastructureRole, FleetSubnetRootReleaseSetManifest,
    fixture::load_fixture_artifact_manifest, load_persisted_application_artifact_union,
    load_persisted_canic_infrastructure_artifact_manifest,
    load_persisted_current_release_set_manifest,
};
use std::path::PathBuf;

use crate::pic::blob_service::artifacts::{self, Composition};

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "ordered production binding and recovery journey"
)]
pub(in crate::pic::fleet_registry::baseline::tests) fn production_blob_initialization_recovers_target_binding()
 {
    if artifacts::write_worker_result(Composition::Dedicated) {
        return;
    }
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let (config_path, root_wasm, coordinator_wasm, store) = artifacts();
    let pic = build_pic();
    let coordinator = pic.create_canister();
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
    let fixture = install_bootstrapped_root_with_config_and_pool_setup(
        &pic,
        root_wasm.clone(),
        coordinator,
        store,
        BootstrappedRootPlacement {
            canister_pool_maximum_size: None,
            canister_pool_minimum_size: None,
            canister_pool_cycles: None,
            coordinator_subnet: None,
            existing_root: None,
            existing_wasm_store: None,
            root_subnet: None,
            component_admission_limits: None,
            fleet_id: None,
            funding: None,
            coordinator_root_funding: None,
        },
        &config_path,
        create_prepaid_pool_assets,
    );
    let operation = [0xd5; 32];
    begin_fixture_fresh_component_provisioning_with_config(
        &pic,
        coordinator,
        coordinator_wasm,
        &fixture,
        operation,
        &config_path,
    );
    await_root_provisioning(&pic, fixture.root_id, operation, |status| {
        status.claimed_component_count == 1 && status.installed_component_count == 0
    });
    let (service, member_operation) = root_pool_status(&pic, fixture.root_id)
        .entries
        .into_iter()
        .find_map(|entry| match entry.status {
            CanisterPoolAssetStatus::Claimed { claim }
            | CanisterPoolAssetStatus::Workload { claim } => {
                Some((entry.canister_id, claim.operation_id))
            }
            _ => None,
        })
        .expect("allocated Blob target waiting for controller bytes");
    let input = installation(service);
    let request = RootComponentInitializationRequest {
        operation_id: member_operation,
        initialization: ComponentApplicationInitialization {
            target_canister: service,
            arguments: candid::encode_one(&input).expect("encode canonical Blob installation"),
        },
    };
    let before = allocation(&pic, fixture.root_id, member_operation);
    assert!(before.application_initialization.is_none());
    assert!(
        pic.canister_status(service, Some(fixture.root_id))
            .unwrap()
            .module_hash
            .is_none()
    );
    let refused: Result<RootCommandResponseFragment, Error> =
        ::ic_testkit::pic::update_candid_args(
            &pic,
            fixture.root_id,
            service,
            canic::protocol::CANIC_ROOT_COMMAND,
            (Command::BindComponentInitialization(request.clone()),),
            ::canic::testing::CANDID_REPLY_LIMITS,
        )
        .map(|(reply,)| reply)
        .unwrap();
    assert_eq!(
        refused.err().expect("binding refusal").code(),
        canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
    );
    let mut wrong_target = request.clone();
    wrong_target.initialization.target_canister = coordinator;
    let refused: Result<RootCommandResponseFragment, Error> =
        ::ic_testkit::pic::update_candid_args(
            &pic,
            fixture.root_id,
            Principal::anonymous(),
            canic::protocol::CANIC_ROOT_COMMAND,
            (Command::BindComponentInitialization(wrong_target),),
            ::canic::testing::CANDID_REPLY_LIMITS,
        )
        .map(|(reply,)| reply)
        .unwrap();
    assert_eq!(
        refused.err().expect("binding refusal").code(),
        canic::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(allocation(&pic, fixture.root_id, member_operation), before);
    pic.upgrade_canister(fixture.root_id, root_wasm, crate::pic::upgrade_args(), None)
        .unwrap();
    assert_eq!(allocation(&pic, fixture.root_id, member_operation), before);
    // Do not consume the ingress reply; reconcile the exact persisted binding.
    let _lost_reply = pic
        .submit_call(
            fixture.root_id,
            Principal::anonymous(),
            canic::protocol::CANIC_ROOT_COMMAND,
            canic_host::component_initialization::encode_command(&request).unwrap(),
        )
        .unwrap();
    for _ in 0..120 {
        if allocation(&pic, fixture.root_id, member_operation)
            .application_initialization
            .is_some()
        {
            break;
        }
        pic.tick();
    }
    assert_eq!(
        allocation(&pic, fixture.root_id, member_operation).application_initialization,
        Some(request.initialization.clone())
    );
    let mut conflicting = request.clone();
    let mut changed = input.clone();
    changed.project = "different-blob-project".into();
    conflicting.initialization.arguments = candid::encode_one(changed).unwrap();
    let refused: Result<RootCommandResponseFragment, Error> =
        ::ic_testkit::pic::update_candid_args(
            &pic,
            fixture.root_id,
            Principal::anonymous(),
            canic::protocol::CANIC_ROOT_COMMAND,
            (Command::BindComponentInitialization(conflicting),),
            ::canic::testing::CANDID_REPLY_LIMITS,
        )
        .map(|(reply,)| reply)
        .unwrap();
    assert_eq!(
        refused.err().expect("binding refusal").code(),
        canic::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    await_root_provisioning(&pic, fixture.root_id, operation, |status| {
        status.root_runtime_active
    });
    let installed: Result<HostConfigurationView, HostFailure> =
        ::ic_testkit::pic::query_candid_args(
            &pic,
            service,
            input.configuration.operator,
            "blob_configuration",
            (),
            ::canic::testing::CANDID_REPLY_LIMITS,
        )
        .map(|(reply,)| reply)
        .unwrap();
    let installed = installed.expect("Root-installed service configuration");
    assert_eq!(installed.configuration, input.configuration);
    assert_eq!(
        allocation(&pic, fixture.root_id, member_operation).phase,
        RootComponentAllocationPhase::Committed
    );
    let version = pic
        .canister_status(service, Some(fixture.root_id))
        .unwrap()
        .version;
    let replay: Result<RootCommandResponseFragment, Error> = ::ic_testkit::pic::update_candid_args(
        &pic,
        fixture.root_id,
        Principal::anonymous(),
        canic::protocol::CANIC_ROOT_COMMAND,
        (Command::BindComponentInitialization(request.clone()),),
        ::canic::testing::CANDID_REPLY_LIMITS,
    )
    .map(|(reply,)| reply)
    .unwrap();
    replay.unwrap();
    assert_eq!(
        pic.canister_status(service, Some(fixture.root_id))
            .unwrap()
            .version,
        version
    );
    assert_eq!(
        allocation(&pic, fixture.root_id, member_operation).application_initialization,
        Some(request.initialization)
    );
}

fn consumer_paths() -> (PathBuf, PathBuf) {
    Composition::Dedicated.paths()
}

fn artifacts() -> (PathBuf, Vec<u8>, Vec<u8>, RootStoreFixture) {
    let (consumer, config_path) = consumer_paths();
    let config = AppConfigSnapshot::load(&config_path).unwrap();
    let build = artifacts::release_build(crate::pic::cases::rust_test_path(
        &production_blob_initialization_recovers_target_binding,
    ));
    let current = load_persisted_current_release_set_manifest(&consumer, build).unwrap();
    canic_host::release_build::validate_finalized_release_build_manifest(
        &consumer,
        build,
        &current.path,
    )
    .unwrap();
    let topology = config.component_topology();
    let application =
        load_persisted_application_artifact_union(&consumer, topology, build).unwrap();
    let infrastructure =
        load_persisted_canic_infrastructure_artifact_manifest(&consumer, build).unwrap();
    let fixtures = load_fixture_artifact_manifest(
        &consumer,
        topology,
        build,
        current.manifest.fixture_artifact_manifest_sha256,
    )
    .unwrap();
    assert_eq!(
        application.digest,
        current.manifest.application_artifact_union_sha256
    );
    assert_eq!(
        infrastructure.digest,
        current.manifest.infrastructure_artifact_manifest_sha256
    );
    assert_eq!(
        fixtures.digest,
        current.manifest.fixture_artifact_manifest_sha256
    );
    let read = |path: &str, size: u64, checksum: &str| {
        let bytes = std::fs::read(consumer.join(path)).unwrap();
        assert_eq!(bytes.len() as u64, size);
        assert_eq!(hex_bytes(wasm_hash(&bytes)), checksum);
        bytes
    };
    let infrastructure_wasm = |role| {
        let entry = infrastructure
            .manifest
            .entries
            .iter()
            .find(|entry| entry.role == role)
            .unwrap();
        read(
            &entry.wasm_relative_path,
            entry.wasm_size_bytes,
            &entry.wasm_sha256_hex,
        )
    };
    let root = infrastructure_wasm(CanicInfrastructureRole::FleetSubnetRoot);
    let coordinator = infrastructure_wasm(CanicInfrastructureRole::FleetCoordinator);
    let store = infrastructure_wasm(CanicInfrastructureRole::WasmStore);
    let manifest = root_manifest(
        &config,
        &config_path,
        &root,
        &store,
        &application.union,
        &fixtures.manifest,
    );
    let artifacts = application
        .union
        .entries
        .iter()
        .map(|entry| {
            read(
                &entry.wasm_relative_path,
                entry.wasm_size_bytes,
                &entry.wasm_sha256_hex,
            );
            (
                entry.role.clone(),
                read(
                    &entry.wasm_gz_relative_path,
                    entry.wasm_gz_size_bytes,
                    &entry.wasm_gz_sha256_hex,
                ),
            )
        })
        .collect();
    (
        config_path,
        root,
        coordinator,
        RootStoreFixture {
            wasm: Some(store),
            manifest,
            artifacts,
        },
    )
}

fn installation(service: Principal) -> ServiceInstallationInput {
    let actor = |byte| Principal::from_slice(&[byte; 29]);
    ServiceInstallationInput {
        configuration: ServiceConfigurationInput {
            service,
            operator: actor(7),
            payment_account: actor(8),
            namespace: 7,
            resources: ServiceResourceInput {
                max_tenants: 1,
                max_object_bytes: 10,
                max_headers: 8,
                max_header_bytes: 1024,
                max_chunks: 4,
                max_tenant_chunks: 2,
                max_objects: 4,
                max_tenant_objects: 2,
                max_physical_bytes: 30_000,
                max_liability_bytes: 20_000,
                max_tenant_logical_bytes: 10_000,
                max_references_per_object: 2,
                max_receipts_per_object: 3,
                max_active: 4,
                max_tenant_active: 2,
            },
            billing: ServiceBillingInput {
                cashier: actor(10),
                reserve: 100,
                minimum_balance: 100,
                target_balance: 200,
                max_gateway_entries: 8,
                max_gateway_unique: 4,
            },
            funding: ServiceFundingInput {
                allocated: 10_000,
                renewal_ceiling: 10_000,
                reserve: 100,
                max_attempts: 4,
            },
            reads: ServiceReadInput {
                sessions: 2,
                tenant_sessions: 1,
                reply_bytes: 2048,
                bytes: 4096,
                tenant_bytes: 2048,
            },
        },
        project: "canic-production-root-blob".into(),
        completion_verifier: actor(11),
    }
}

fn root_manifest(
    config: &AppConfigSnapshot,
    config_path: &Path,
    root: &[u8],
    store: &[u8],
    application: &canic_host::release_set::ApplicationArtifactUnion,
    fixtures: &canic_host::release_set::fixture::FixtureArtifactManifest,
) -> RootStoreReleaseSetManifest {
    let projection: FleetSubnetRootInitArgs = decode_one(
        &install_root_args_with_release_set_digest_and_coordinator(ManagedRootInstallInput {
            root_id: Principal::anonymous(),
            wasm_store: Principal::anonymous(),
            installation_controller: Principal::anonymous(),
            coordinator: Principal::anonymous(),
            root_wasm: root,
            wasm_store_wasm: store,
            config_path,
            release_set_digest: ReleaseSetDigest::from_bytes([0; 32]),
        })
        .unwrap(),
    )
    .unwrap();
    FleetSubnetRootReleaseSetManifest::project(
        config.component_topology(),
        &projection.authority.binding,
        application,
        fixtures,
    )
    .unwrap()
    .root_store_manifest()
}
