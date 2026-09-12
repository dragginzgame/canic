//! Real Store-to-IcyDB delivery through the shipped registered importer contract.

mod in_flight;
mod reinstall;

use super::{descriptor, grant, prepare, upload};
use crate::fixture_provisioning::{first_row, progress, restart, row_bytes};
use crate::{configure_runtime, drive_icydb_startup, wait_for_canic_install_callback};
use candid::{CandidType, Deserialize, Principal, encode_one};
use canic::{
    Error,
    dto::{
        fixture_provisioning::{
            FixtureChunkUpload, FixtureGrantRequest, FixtureImportError, FixtureImportFailure,
            FixtureProvisioningStatus,
        },
        fleet_subnet_root::FleetSubnetWasmStoreInitArgs,
    },
    ids::{FleetSubnetWasmStoreAuthority, ManagedCanisterBinding},
};
use canic_testing_internal::pic::{
    CanicIcydbLifecycleFixture, InstalledFixtureConsumer, install_canic_icydb_lifecycle_fixture,
    retained_fixture_store_wasm,
};
use ic_testkit::pic::{CandidCallExt, PocketIc};
use sha2::{Digest, Sha256};
use std::time::Duration;

#[derive(CandidType, Clone, Copy)]
enum ConsumerFault {
    None,
    ErrorAfterRows,
    SkipCheckpoint,
    BadReceipt,
    PauseAfterFirstChunk,
    PauseBeforeFetch,
    WrongAuthority,
}

#[test]
fn automatic_consumer_recovers_outages_traps_and_restart_then_stops_on_durable_receipt() {
    let wasm = retained_fixture_store_wasm();
    let fixture = install_canic_icydb_lifecycle_fixture();
    let pic = &fixture.pic;
    let store = pic.create_canister();
    pic.add_cycles(store, 10_000_000_000_000);
    let chunks = (0..3).map(row_bytes).collect::<Vec<_>>();
    let mut descriptor = descriptor(&chunks);
    descriptor.completion_summary = Sha256::digest(
        descriptor
            .chunks
            .iter()
            .flat_map(|chunk| chunk.digest)
            .collect::<Vec<_>>(),
    )
    .into();
    let target = fixture.install_fixture_consumer(store, descriptor.clone());
    let controller = Principal::from_slice(&[0x46; 29]);
    install_source(
        &fixture,
        store,
        controller,
        &wasm,
        &target,
        &descriptor,
        &chunks,
    );
    let cases = [
        (target, ConsumerFault::ErrorAfterRows),
        (
            fixture.install_fixture_consumer(store, descriptor.clone()),
            ConsumerFault::SkipCheckpoint,
        ),
        (
            fixture.install_fixture_consumer(store, descriptor.clone()),
            ConsumerFault::BadReceipt,
        ),
    ];
    for (target, fault) in cases {
        exercise_automatic_target(&fixture, store, controller, target, fault);
    }

    reinstall::qualify(&fixture, store, controller, &wasm, &descriptor);
    qualify_malformed_reply(&fixture, &descriptor);

    let rejected = fixture.install_fixture_consumer(store, descriptor);
    pic.stop_canister(store, Some(controller)).unwrap();
    configure_runtime(pic, rejected.canister_id, fixture.root, rejected.directory);
    set_fault(pic, rejected.canister_id, ConsumerFault::WrongAuthority);
    drive(pic, 20);
    assert_eq!(
        status(pic, rejected.canister_id),
        FixtureProvisioningStatus::Failed(FixtureImportFailure::Authority)
    );
    assert_application_gate(pic, rejected.canister_id, false);
    restart(&fixture, rejected.canister_id);
    drive(pic, 20);
    assert_eq!(
        status(pic, rejected.canister_id),
        FixtureProvisioningStatus::Failed(FixtureImportFailure::Authority)
    );
    assert_application_gate(pic, rejected.canister_id, false);
}

fn install_source(
    fixture: &CanicIcydbLifecycleFixture,
    store: Principal,
    controller: Principal,
    wasm: &[u8],
    target: &InstalledFixtureConsumer,
    descriptor: &canic::dto::fixture_provisioning::FixtureDescriptor,
    chunks: &[Vec<u8>],
) {
    let pic = &fixture.pic;
    let ManagedCanisterBinding::Component(component) = &target.assignment.grant.binding.target
    else {
        unreachable!()
    };
    pic.set_controllers(store, None, vec![controller, fixture.root])
        .unwrap();
    pic.install_canister(
        store,
        wasm.to_vec(),
        encode_one(FleetSubnetWasmStoreInitArgs {
            authority: FleetSubnetWasmStoreAuthority {
                authority: component.authority.clone(),
                placement_subnet: component.placement_subnet,
                fleet_subnet_root: fixture.root,
                wasm_store: store,
                installation_controller: controller,
                release_build_id: target.assignment.grant.binding.release_build_id,
                wasm_module_hash: Sha256::digest(wasm).into(),
            },
            install_id: [0x31; 32],
        })
        .unwrap(),
        Some(controller),
    );
    prepare(pic, store, fixture.root, descriptor.clone()).unwrap();
    for (index, bytes) in chunks.iter().enumerate() {
        upload(
            pic,
            store,
            controller,
            FixtureChunkUpload {
                content_id: target.assignment.grant.binding.content_id,
                index: u32::try_from(index).unwrap(),
                bytes: bytes.clone(),
            },
        )
        .unwrap();
    }
}

fn qualify_malformed_reply(
    fixture: &CanicIcydbLifecycleFixture,
    descriptor: &canic::dto::fixture_provisioning::FixtureDescriptor,
) {
    let pic = &fixture.pic;
    let (source, _) = fixture.install_composed_canister();
    let target = fixture.install_fixture_consumer(source, descriptor.clone());
    configure_runtime(pic, target.canister_id, fixture.root, target.directory);
    wait_for_canic_install_callback(pic, target.canister_id);
    drive_icydb_startup(pic, target.canister_id);
    drive(pic, 40);
    let expected = FixtureProvisioningStatus::Failed(FixtureImportFailure::Codec {
        code: Error::from_registered(canic_core::diagnostics::codes::CODEC_INVALID).raw_code(),
    });
    assert_eq!(status(pic, target.canister_id), expected);
    assert_eq!(progress(fixture, target.canister_id).next, 0);
    assert!(first_row(fixture, target.canister_id).is_empty());
    assert_application_gate(pic, target.canister_id, false);
    restart(fixture, target.canister_id);
    drive(pic, 80);
    assert_eq!(status(pic, target.canister_id), expected);
    assert_application_gate(pic, target.canister_id, false);
    let reads: u64 = pic
        .query_candid(source, "fixture_malformed_reads", ())
        .unwrap();
    assert_eq!(
        reads, 1,
        "permanent codec failure must not retry after restart"
    );
}

fn exercise_automatic_target(
    fixture: &CanicIcydbLifecycleFixture,
    store: Principal,
    controller: Principal,
    target: InstalledFixtureConsumer,
    fault: ConsumerFault,
) {
    let pic = &fixture.pic;
    let canister = target.canister_id;
    let expected = target.assignment.grant.clone();
    assert_eq!(
        grant(
            pic,
            store,
            fixture.root,
            FixtureGrantRequest {
                expected_revision: 0,
                binding: expected.binding.clone(),
                enabled: true,
            }
        )
        .unwrap(),
        expected
    );
    pic.stop_canister(store, Some(controller)).unwrap();
    configure_runtime(pic, canister, fixture.root, target.directory);
    set_fault(pic, canister, fault);
    wait_for_canic_install_callback(pic, canister);
    drive_icydb_startup(pic, canister);
    drive(pic, 40);
    assert_eq!(progress(fixture, canister).next, 0);
    assert!(first_row(fixture, canister).is_empty());
    assert_application_gate(pic, canister, false);
    pic.start_canister(store, Some(controller)).unwrap();
    drive(pic, 80);
    let stalled = progress(fixture, canister);
    assert!(stalled.receipt.is_none());
    assert_application_gate(pic, canister, false);
    match fault {
        ConsumerFault::BadReceipt => assert_eq!(stalled.next, 3),
        ConsumerFault::ErrorAfterRows | ConsumerFault::SkipCheckpoint => {
            assert_eq!(stalled.next, 0);
            assert!(first_row(fixture, canister).is_empty());
        }
        _ => unreachable!(),
    }
    // Upgrade reconstructs demand without an operator reissuing a delivery step.
    if matches!(fault, ConsumerFault::SkipCheckpoint) {
        restart(fixture, canister);
        drive_icydb_startup(pic, canister);
    } else {
        set_fault(pic, canister, ConsumerFault::None);
    }
    let complete = wait_complete(pic, canister);
    let FixtureProvisioningStatus::Complete(receipt) = &complete else {
        unreachable!()
    };
    assert_eq!(receipt.binding, expected.binding);
    assert_application_gate(pic, canister, true);
    assert_eq!(
        receipt.completion_summary,
        target.assignment.descriptor.completion_summary
    );
    grant(
        pic,
        store,
        fixture.root,
        FixtureGrantRequest {
            expected_revision: 1,
            binding: expected.binding,
            enabled: false,
        },
    )
    .unwrap();
    drive(pic, 20);
    assert_eq!(status(pic, canister), complete);
    restart(fixture, canister);
    drive_icydb_startup(pic, canister);
    drive(pic, 20);
    assert_eq!(status(pic, canister), complete);
    assert_application_gate(pic, canister, true);
}

fn drive(pic: &PocketIc, rounds: usize) {
    for _ in 0..rounds {
        pic.advance_time(Duration::from_secs(5));
        pic.tick();
    }
}

fn wait_complete(pic: &PocketIc, canister: Principal) -> FixtureProvisioningStatus {
    for _ in 0..160 {
        if let complete @ FixtureProvisioningStatus::Complete(_) = status(pic, canister) {
            return complete;
        }
        drive(pic, 1);
    }
    panic!(
        "automatic fixture delivery did not complete: {:?}",
        status(pic, canister)
    );
}

fn status(pic: &PocketIc, target: Principal) -> FixtureProvisioningStatus {
    let result: Result<Result<FixtureProvisioningStatus, FixtureImportError>, Error> = pic
        .query_candid(target, "fixture_consumer_status", ())
        .unwrap();
    result.unwrap().unwrap()
}

fn set_fault(pic: &PocketIc, target: Principal, fault: ConsumerFault) {
    let result: Result<(), Error> = pic
        .update_candid(target, "fixture_consumer_fault", (fault,))
        .unwrap();
    result.unwrap();
}

#[derive(CandidType)]
enum ReadinessRequest {
    Readiness,
}

#[derive(CandidType, Deserialize)]
enum ReadinessResponse {
    Readiness(canic::dto::runtime::CanicReadinessStatus),
}

fn assert_application_gate(pic: &PocketIc, target: Principal, ready: bool) {
    let response: Result<ReadinessResponse, Error> = pic
        .query_candid(
            target,
            canic::protocol::CANIC_OBSERVABILITY,
            (ReadinessRequest::Readiness,),
        )
        .unwrap();
    let ReadinessResponse::Readiness(response) = response.unwrap();
    assert_eq!(
        response.status == canic::dto::runtime::ReadinessStatus::Ready,
        ready
    );
    assert_eq!(
        matches!(response.fixture, Ok(FixtureProvisioningStatus::Complete(_))),
        ready
    );
    let query = pic.query_candid::<Result<Principal, Error>, _>(
        target,
        "composed_framework_public_probe",
        (),
    );
    if ready {
        assert_eq!(query.unwrap().unwrap(), Principal::anonymous());
    } else {
        assert!(
            query.is_err(),
            "application query must not dispatch while fixture is pending"
        );
        let update = pic.update_candid::<Result<Principal, Error>, _>(
            target,
            "composed_framework_owned_probe",
            (),
        );
        assert!(
            update.is_err(),
            "application update must be rejected before its handler returns"
        );
    }
}
