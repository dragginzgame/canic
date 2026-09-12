//! Actual retained Store endpoints, shared bytes and exact grant recovery in PocketIC.

mod consumer;
mod retirement;

use super::{configure_runtime, drive_icydb_startup, wait_for_canic_install_callback};
use std::time::Duration;

use candid::{Principal, encode_one};
use canic::{
    Error,
    dto::{
        fixture_provisioning::{
            FixtureChunkDescriptor, FixtureChunkRead, FixtureChunkUpload, FixtureDescriptor,
            FixtureGrant, FixtureGrantRequest, FixtureSourceStatus, FixtureStoreError,
            FixtureTargetBinding,
        },
        fleet_subnet_root::FleetSubnetWasmStoreInitArgs,
    },
    ids::FleetSubnetWasmStoreAuthority,
    protocol::{CANIC_WASM_STORE_COMMAND, CANIC_WASM_STORE_PUBLISH_FIXTURE},
};
use canic_control_plane::{
    api::fixture_content::FixtureContentApi,
    dto::template::{StoreCommand, StoreCommandResponse},
};
use canic_core::ids::{ManagedCanisterBinding, ReleaseBuildId, ReleaseBuildNonce};
use canic_testing_internal::pic::{
    install_canic_icydb_lifecycle_fixture, retained_fixture_store_wasm, upgrade_args,
};
use ic_testkit::pic::{CandidCallExt, CanisterInstallExt, PocketIc};
use sha2::{Digest, Sha256};

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one actual Store retains the upload, restart and target-grant revocation journey"
)]
fn retained_store_recovers_upload_and_fences_revoked_target_reads() {
    let wasm = retained_fixture_store_wasm();
    let fixture = install_canic_icydb_lifecycle_fixture();
    let (target, directory) = fixture.install_composed_canister();
    let component = directory.authority.component.provenance.component.clone();
    configure_runtime(&fixture.pic, target, fixture.root, directory);
    wait_for_canic_install_callback(&fixture.pic, target);
    drive_icydb_startup(&fixture.pic, target);
    let pic = &fixture.pic;
    let store = pic.create_canister();
    pic.add_cycles(store, 10_000_000_000_000);
    let controller = Principal::from_slice(&[0x46; 29]);
    let authority = FleetSubnetWasmStoreAuthority {
        authority: component.authority.clone(),
        placement_subnet: component.placement_subnet,
        fleet_subnet_root: fixture.root,
        wasm_store: store,
        installation_controller: controller,
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
            [0x11; 32],
        )),
        wasm_module_hash: Sha256::digest(&wasm).into(),
    };
    pic.set_controllers(store, None, vec![controller, fixture.root])
        .unwrap();
    pic.install_canister(
        store,
        wasm.clone(),
        encode_one(FleetSubnetWasmStoreInitArgs {
            authority: authority.clone(),
            install_id: [0x31; 32],
        })
        .unwrap(),
        Some(controller),
    );

    let bytes = vec![vec![5; canic_core::CANIC_WASM_CHUNK_BYTES], vec![6; 64]];
    let descriptor = descriptor(&bytes);
    let content_id = FixtureContentApi::content_id(&descriptor).unwrap();
    assert_eq!(
        upload(
            pic,
            store,
            controller,
            FixtureChunkUpload {
                content_id,
                index: 0,
                bytes: bytes[0].clone(),
            }
        ),
        Err(FixtureStoreError::NotFound)
    );
    let prepared = prepare(pic, store, fixture.root, descriptor.clone()).unwrap();
    assert!(!prepared.complete);
    let denied: Result<StoreCommandResponse, Error> = pic
        .update_candid_as(
            store,
            controller,
            CANIC_WASM_STORE_COMMAND,
            (StoreCommand::PrepareFixture(descriptor.clone()),),
        )
        .unwrap();
    assert!(
        denied.is_err(),
        "installation controller cannot publish fixture authority"
    );

    let request = FixtureGrantRequest {
        expected_revision: 0,
        enabled: true,
        binding: FixtureTargetBinding {
            target: ManagedCanisterBinding::Component(component),
            installation: [0x32; 32],
            release_build_id: authority.release_build_id,
            content_id,
        },
    };
    assert_eq!(
        grant(pic, store, fixture.root, request.clone()),
        Err(FixtureStoreError::NotReady)
    );
    let first = FixtureChunkUpload {
        content_id,
        index: 0,
        bytes: bytes[0].clone(),
    };
    let denied: Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> = pic
        .update_candid_as(
            store,
            Principal::anonymous(),
            CANIC_WASM_STORE_PUBLISH_FIXTURE,
            (first.clone(),),
        )
        .unwrap();
    assert!(denied.is_err());
    pic.set_controllers(store, Some(controller), vec![fixture.root])
        .unwrap();
    let denied: Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> = pic
        .update_candid_as(
            store,
            controller,
            CANIC_WASM_STORE_PUBLISH_FIXTURE,
            (first.clone(),),
        )
        .unwrap();
    assert!(
        denied.is_err(),
        "retained identity must still be an observed controller"
    );
    pic.set_controllers(store, Some(fixture.root), vec![controller, fixture.root])
        .unwrap();
    let status = upload(pic, store, controller, first.clone()).unwrap();
    assert_eq!(status.next_chunk, 1);
    restart(pic, store, controller, &wasm);
    assert_eq!(
        prepare(pic, store, fixture.root, descriptor).unwrap(),
        status
    );
    assert_eq!(upload(pic, store, controller, first).unwrap(), status);
    assert_eq!(
        upload(
            pic,
            store,
            fixture.root,
            FixtureChunkUpload {
                content_id,
                index: 1,
                bytes: vec![9; 64],
            }
        ),
        Err(FixtureStoreError::Content)
    );
    assert!(
        upload(
            pic,
            store,
            fixture.root,
            FixtureChunkUpload {
                content_id,
                index: 1,
                bytes: bytes[1].clone(),
            }
        )
        .unwrap()
        .complete
    );

    let selected = grant(pic, store, fixture.root, request.clone()).unwrap();
    let read = FixtureChunkRead {
        grant: selected.clone(),
        index: 0,
    };
    let denied: Result<Result<Vec<u8>, FixtureStoreError>, Error> = pic
        .update_candid_as(
            store,
            controller,
            canic::protocol::CANIC_WASM_STORE_FIXTURE_CHUNK,
            (read.clone(),),
        )
        .unwrap();
    assert_eq!(denied.unwrap(), Err(FixtureStoreError::Authority));
    assert_eq!(pull(pic, target, store, read.clone()).unwrap(), bytes[0]);
    restart(pic, store, controller, &wasm);
    assert_eq!(
        grant(pic, store, fixture.root, request.clone()).unwrap(),
        selected
    );
    assert_eq!(pull(pic, target, store, read.clone()).unwrap(), bytes[0]);

    let revoke = FixtureGrantRequest {
        expected_revision: selected.revision,
        enabled: false,
        ..request.clone()
    };
    let revoked = grant(pic, store, fixture.root, revoke.clone()).unwrap();
    assert_eq!(grant(pic, store, fixture.root, revoke).unwrap(), revoked);
    assert_eq!(
        grant(pic, store, fixture.root, request.clone()),
        Err(FixtureStoreError::Conflict)
    );
    assert_eq!(
        pull(pic, target, store, read.clone()),
        Err(FixtureStoreError::Authority)
    );
    let mut replacement = request;
    replacement.expected_revision = revoked.revision;
    replacement.binding.installation = [0x33; 32];
    let selected = grant(pic, store, fixture.root, replacement).unwrap();
    assert_eq!(
        pull(pic, target, store, read),
        Err(FixtureStoreError::Authority)
    );
    assert_eq!(
        pull(
            pic,
            target,
            store,
            FixtureChunkRead {
                grant: selected.clone(),
                index: 1
            }
        )
        .unwrap(),
        bytes[1]
    );
    retirement::qualify(
        pic,
        store,
        fixture.root,
        controller,
        &wasm,
        target,
        FixtureChunkRead {
            grant: selected,
            index: 1,
        },
    );
}

fn descriptor(chunks: &[Vec<u8>]) -> FixtureDescriptor {
    FixtureDescriptor {
        schema_version: 1,
        format_hash: [0x34; 32],
        encoded_length: chunks.iter().map(|chunk| chunk.len() as u64).sum(),
        chunks: chunks
            .iter()
            .map(|chunk| FixtureChunkDescriptor {
                digest: Sha256::digest(chunk).into(),
                length: u32::try_from(chunk.len()).unwrap(),
            })
            .collect(),
        completion_summary: [0x35; 32],
    }
}

fn prepare(
    pic: &PocketIc,
    store: Principal,
    root: Principal,
    descriptor: FixtureDescriptor,
) -> Result<FixtureSourceStatus, FixtureStoreError> {
    let result: Result<StoreCommandResponse, Error> = pic
        .update_candid_as(
            store,
            root,
            CANIC_WASM_STORE_COMMAND,
            (StoreCommand::PrepareFixture(descriptor),),
        )
        .unwrap();
    let StoreCommandResponse::FixtureSource(status) = result.unwrap() else {
        panic!("wrong Store command correlation");
    };
    status
}

fn upload(
    pic: &PocketIc,
    store: Principal,
    root: Principal,
    request: FixtureChunkUpload,
) -> Result<FixtureSourceStatus, FixtureStoreError> {
    let result: Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> = pic
        .update_candid_as(store, root, CANIC_WASM_STORE_PUBLISH_FIXTURE, (request,))
        .unwrap();
    result.unwrap()
}

fn grant(
    pic: &PocketIc,
    store: Principal,
    root: Principal,
    request: FixtureGrantRequest,
) -> Result<FixtureGrant, FixtureStoreError> {
    let result: Result<StoreCommandResponse, Error> = pic
        .update_candid_as(
            store,
            root,
            CANIC_WASM_STORE_COMMAND,
            (StoreCommand::SetFixtureGrant(Box::new(request)),),
        )
        .unwrap();
    let StoreCommandResponse::FixtureGrant(grant) = result.unwrap() else {
        panic!("wrong Store command correlation");
    };
    *grant
}

fn pull(
    pic: &PocketIc,
    target: Principal,
    store: Principal,
    request: FixtureChunkRead,
) -> Result<Vec<u8>, FixtureStoreError> {
    let result: Result<Result<Vec<u8>, FixtureStoreError>, Error> = pic
        .update_candid(target, "fixture_read_retained_source", (store, request))
        .unwrap();
    result.unwrap()
}

fn restart(pic: &PocketIc, store: Principal, controller: Principal, wasm: &[u8]) {
    pic.wait_out_install_code_rate_limit(Duration::from_mins(5));
    pic.upgrade_canister(store, wasm.to_vec(), upgrade_args(), Some(controller))
        .unwrap();
}
