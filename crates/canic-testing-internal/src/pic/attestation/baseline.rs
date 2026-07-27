use candid::Principal;
use ic_testkit::pic::{CachedPicBaseline, Pic, restore_or_rebuild_cached_pic_baseline};
use std::{
    path::Path,
    sync::{Mutex, OnceLock},
};

use crate::pic::{
    canic::{activate_managed_fleet, install_root_args},
    role_pid as lookup_role_pid, wait_until_ready as wait_for_ready_canister,
};

use super::{
    build::{build_pic, build_test_root_wasm, root_canister_config_path},
    fixture::{CachedInstalledRoot, progress},
};

const ROOT_INSTALL_CYCLES: u128 = 80_000_000_000_000;
static ROOT_ISSUER_BASELINE: OnceLock<
    Mutex<Option<CachedPicBaseline<AttestationBaselineMetadata>>>,
> = OnceLock::new();

pub struct AttestationBaselineMetadata {
    root_id: Principal,
    wasm_store_id: Principal,
    issuer_id: Principal,
}

struct InstalledRoot {
    pic: super::build::SerialPic,
    root_id: Principal,
}

// Restore or create the cached `root + issuer` baseline.
#[must_use]
pub(super) fn install_cached_root_fixture() -> CachedInstalledRoot {
    progress("request cached root+issuer baseline");
    let baseline_slot = ROOT_ISSUER_BASELINE.get_or_init(|| Mutex::new(None));
    let (baseline, cache_hit) = restore_or_rebuild_cached_pic_baseline(
        baseline_slot,
        build_cached_baseline,
        restore_cached_baseline,
    );
    if cache_hit {
        progress("cache hit");
    }
    progress("cached fixture restore complete");

    CachedInstalledRoot {
        root_id: baseline.metadata().root_id,
        issuer_id: baseline.metadata().issuer_id,
        pic: baseline,
    }
}

// Resolve the issuer canister from the root-managed subnet registry.
#[must_use]
fn issuer_pid(pic: &Pic, root_id: Principal) -> Principal {
    lookup_role_pid(pic, root_id, "issuer", 120)
}

// Resolve the managed wasm_store canister from the root-managed subnet registry.
#[must_use]
fn wasm_store_pid(pic: &Pic, root_id: Principal) -> Principal {
    lookup_role_pid(pic, root_id, "wasm_store", 120)
}

// Build one reusable baseline and capture immutable snapshot IDs inside it.
fn build_cached_baseline() -> CachedPicBaseline<AttestationBaselineMetadata> {
    progress("cache miss, building fresh baseline");
    let InstalledRoot { pic, root_id } = install_test_root();
    progress("waiting for issuer registration");
    let issuer_id = issuer_pid(&pic, root_id);
    wait_for_ready_canister(&pic, issuer_id, 240);
    let wasm_store_id = wasm_store_pid(&pic, root_id);
    wait_for_ready_canister(&pic, wasm_store_id, 240);
    progress("issuer ready");

    progress("waiting for root readiness before snapshot capture");
    wait_for_ready_canister(&pic, root_id, 240);
    progress("capturing baseline snapshots");
    let controller_ids = vec![root_id, wasm_store_id, issuer_id];
    let baseline = CachedPicBaseline::capture(
        pic.into_pic(),
        root_id,
        controller_ids,
        AttestationBaselineMetadata {
            root_id,
            wasm_store_id,
            issuer_id,
        },
    )
    .expect("downloaded baseline snapshots unavailable");
    progress("fresh baseline ready");
    baseline
}

// Restore the cached baseline snapshots into the same baseline PocketIC instance.
fn restore_cached_baseline(baseline: &CachedPicBaseline<AttestationBaselineMetadata>) {
    progress("restoring cached baseline snapshots");
    baseline.restore(baseline.metadata().root_id);

    baseline.pic().tick();

    progress("waiting for restored root and issuer readiness");
    wait_for_ready_canister(baseline.pic(), baseline.metadata().wasm_store_id, 240);
    wait_for_ready_canister(baseline.pic(), baseline.metadata().issuer_id, 240);
    wait_for_ready_canister(baseline.pic(), baseline.metadata().root_id, 240);
}

// Install the test root into a fresh PocketIC instance.
fn install_test_root() -> InstalledRoot {
    install_root_fixture(build_test_root_wasm())
}

// Install one root wasm into a fresh serialized PocketIC instance.
fn install_root_fixture(root_wasm: Vec<u8>) -> InstalledRoot {
    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);
    activate_test_fleet(&pic, root_id);

    InstalledRoot { pic, root_id }
}

fn activate_test_fleet(pic: &Pic, root_id: Principal) {
    progress("preparing Fleet activation");
    progress("activating prepared Fleet");
    activate_managed_fleet(pic, root_id);
}

// Install the root canister under PocketIC with the current exact Fleet identity.
fn install_root_canister(pic: &Pic, wasm: Vec<u8>) -> Principal {
    let root_id = pic.create_canister();
    pic.add_cycles(root_id, ROOT_INSTALL_CYCLES);
    let config_path = root_canister_config_path(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root"),
    );
    let init_args =
        install_root_args(root_id, &wasm, &config_path).expect("encode root install identity");
    pic.install_canister(root_id, wasm, init_args, None);
    root_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{decode_one, encode_one};
    use canic::{
        CANIC_WASM_CHUNK_BYTES,
        dto::{
            fleet_registry::{
                FleetSubnetRootEntry, FleetSubnetRootJoinRequest, FleetSubnetRootJoinResponse,
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
        ids::{CanisterRole, ReleaseSetDigest},
    };
    use canic::{
        Error,
        dto::fleet_activation::{FleetActivationPhase, FleetActivationStatusResponse},
        protocol::{
            CANIC_FLEET_ACTIVATION_STATUS, CANIC_FLEET_REGISTRY_ROOT_ACKNOWLEDGEMENTS,
            CANIC_FLEET_REGISTRY_SYNC_STATUS, CANIC_FLEET_REGISTRY_SYNCHRONIZE,
            CANIC_FLEET_REGISTRY_VERSION, CANIC_FLEET_SUBNET_ROOT_AUTHORITY,
            CANIC_FLEET_SUBNET_ROOT_JOIN, CANIC_ROOT_STORE_BOOTSTRAP,
            CANIC_ROOT_STORE_BOOTSTRAP_STATUS, CANIC_TEMPLATE_PREPARE_ADMIN,
            CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN, CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
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
    use std::time::Duration;

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
    fn prepared_root_upgrade_does_not_run_runtime_or_application_continuations() {
        let root_wasm = build_test_root_wasm();
        let pic = build_pic();
        let root_id = install_root_canister(&pic, root_wasm.clone());
        assert_prepared(&pic, root_id);

        pic.wait_out_install_code_rate_limit(Duration::from_mins(5));
        pic.upgrade_canister(
            root_id,
            root_wasm,
            encode_one(()).expect("encode root upgrade"),
            None,
        )
        .expect("upgrade Prepared root");
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
        assert_prepared(&pic, root_id);

        activate_test_fleet(&pic, root_id);
        wait_for_ready_canister(&pic, root_id, 240);
        pic.tick();

        let executions: Result<u64, Error> = pic
            .query_call(root_id, "root_upgrade_hook_executions", ())
            .expect("query root upgrade hook count");
        assert_eq!(
            executions.expect("root upgrade hook count"),
            0,
            "Prepared root upgrade must not run the application upgrade hook"
        );
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
                (sync_request,),
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
        assert_prepared(&pic, fixture.root_id);
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
