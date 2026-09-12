//! Reinstall while the real Store fetch is between subnet rounds.

use super::{
    ConsumerFault, assert_application_gate, descriptor, drive, first_row, grant, install_source,
    progress, row_bytes, set_fault, wait_complete,
};
use crate::{
    configure_runtime, drive_icydb_startup,
    fixture_provisioning::{ImportError, ImportSnapshot},
    wait_for_canic_install_callback,
};
use candid::{CandidType, Principal, encode_one};
use canic::{
    Error,
    dto::fixture_provisioning::{
        FixtureChunkRead, FixtureDescriptor, FixtureGrantRequest, FixtureProvisioningStatus,
        FixtureStoreError,
    },
};
use canic_testing_internal::pic::{
    CanicIcydbLifecycleFixture, InstalledFixtureConsumer, held_fixture_store_wasm,
    install_canic_icydb_lifecycle_fixture_with_builder, retained_fixture_store_wasm,
};
use ic_testkit::pic::{CandidCallExt, CanisterInstallExt, PocketIcBuilder};
use sha2::{Digest, Sha256};
use std::time::Duration;

#[test]
fn real_store_fetch_is_pending_when_consumer_reinstall_is_submitted() {
    let PendingFetchFixture {
        fixture,
        original,
        store,
        descriptor,
    } = prepare_pending_fetch(false);
    let pic = &fixture.pic;
    let target = original.canister_id;
    let release = pic
        .submit_call(
            target,
            Principal::anonymous(),
            "fixture_consumer_fault",
            encode_one(ConsumerFault::None).unwrap(),
        )
        .unwrap();
    let mut pending = false;
    for _ in 0..120 {
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
        pending = pic
            .query_candid(target, "fixture_consumer_fetch_pending", ())
            .unwrap();
        if pending {
            break;
        }
    }
    assert!(
        pending,
        "observe the actual Store fetch before submitting reinstall"
    );
    assert!(pic.ingress_status(release).is_some());
    assert_eq!(progress(&fixture, target).next, 0);
    let replacement = fixture.reinstall_fixture_consumer(&original, [0xb8; 32], 3);
    let empty: Result<Result<ImportSnapshot, ImportError>, Error> =
        pic.query_candid(target, "fixture_progress", ()).unwrap();
    assert!(matches!(empty.unwrap(), Err(ImportError::NotBegun)));
    drive(pic, 10);
    let after_replies: Result<Result<ImportSnapshot, ImportError>, Error> =
        pic.query_candid(target, "fixture_progress", ()).unwrap();
    assert!(matches!(after_replies.unwrap(), Err(ImportError::NotBegun)));
    assert!(first_row(&fixture, target).is_empty());
    assert_application_gate(pic, target, false);
    assert!(
        !pic.query_candid::<bool, _>(target, "fixture_consumer_fetch_pending", ())
            .unwrap()
    );
    grant(
        pic,
        store,
        fixture.root,
        FixtureGrantRequest {
            binding: original.assignment.grant.binding.clone(),
            expected_revision: 1,
            enabled: false,
        },
    )
    .unwrap();
    grant(
        pic,
        store,
        fixture.root,
        FixtureGrantRequest {
            binding: replacement.assignment.grant.binding.clone(),
            expected_revision: 2,
            enabled: true,
        },
    )
    .unwrap();
    configure_runtime(pic, target, fixture.root, replacement.directory);
    wait_for_canic_install_callback(pic, target);
    drive_icydb_startup(pic, target);
    let FixtureProvisioningStatus::Complete(receipt) = wait_complete(pic, target) else {
        panic!("replacement completes");
    };
    assert_eq!(receipt.binding, replacement.assignment.grant.binding);
    assert_eq!(receipt.completion_summary, descriptor.completion_summary);
    assert_eq!(
        crate::fixture_provisioning::store::pull(
            pic,
            target,
            store,
            FixtureChunkRead {
                grant: original.assignment.grant,
                index: 0
            }
        ),
        Err(FixtureStoreError::Authority)
    );
}

struct PendingFetchFixture {
    fixture: CanicIcydbLifecycleFixture,
    original: InstalledFixtureConsumer,
    store: Principal,
    descriptor: FixtureDescriptor,
}

fn prepare_pending_fetch(hold_reply: bool) -> PendingFetchFixture {
    let fixture = install_canic_icydb_lifecycle_fixture_with_builder(
        PocketIcBuilder::new()
            .with_application_subnet()
            .with_application_subnet(),
    );
    let pic = &fixture.pic;
    let routing_probe = pic.create_canister();
    let consumer_subnet = pic.get_subnet(routing_probe).unwrap();
    let source_subnet = pic
        .topology()
        .get_app_subnets()
        .into_iter()
        .find(|subnet| *subnet != consumer_subnet)
        .unwrap();
    let store = pic.create_canister_on_subnet(None, None, source_subnet);
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
    let original = fixture.install_fixture_consumer(store, descriptor.clone());
    let target = original.canister_id;
    assert_ne!(pic.get_subnet(target), pic.get_subnet(store));
    let controller = Principal::from_slice(&[0x47; 29]);
    install_source(
        &fixture,
        store,
        controller,
        &if hold_reply {
            held_fixture_store_wasm()
        } else {
            retained_fixture_store_wasm()
        },
        &original,
        &descriptor,
        &chunks,
    );
    grant(
        pic,
        store,
        fixture.root,
        FixtureGrantRequest {
            binding: original.assignment.grant.binding.clone(),
            expected_revision: 0,
            enabled: true,
        },
    )
    .unwrap();
    set_fault(pic, target, ConsumerFault::PauseBeforeFetch);
    configure_runtime(pic, target, fixture.root, original.directory.clone());
    wait_for_canic_install_callback(pic, target);
    drive_icydb_startup(pic, target);
    pic.wait_out_install_code_rate_limit(Duration::from_mins(5));
    PendingFetchFixture {
        fixture,
        original,
        store,
        descriptor,
    }
}

#[derive(CandidType)]
enum HeldReply {
    Chunk,
}

#[test]
fn actual_store_reply_remains_held_across_consumer_reinstall() {
    let PendingFetchFixture {
        fixture,
        original,
        store,
        descriptor,
    } = prepare_pending_fetch(true);
    let pic = &fixture.pic;
    let target = original.canister_id;
    arm_chunk_reply(&fixture, store);
    set_fault(pic, target, ConsumerFault::None);
    let waiting = || {
        pic.query_candid_as::<u32, _>(store, fixture.root, "canic_test_fixture_reply_waiting", ())
            .unwrap()
    };
    for _ in 0..120 {
        if waiting() == 1 {
            break;
        }
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
    }
    assert_eq!(waiting(), 1);
    assert_eq!(progress(&fixture, target).next, 0);
    assert!(
        pic.query_candid::<bool, _>(target, "fixture_consumer_fetch_pending", ())
            .unwrap()
    );
    let replacement = fixture.reinstall_fixture_consumer(&original, [0xc8; 32], 3);
    assert_eq!(
        waiting(),
        1,
        "real Store response stays withheld until reinstall completes"
    );
    pic.update_candid_as::<(), _>(store, fixture.root, "canic_test_fixture_reply_release", ())
        .unwrap();
    drive(pic, 10);
    assert_eq!(waiting(), 0);
    let empty: Result<Result<ImportSnapshot, ImportError>, Error> =
        pic.query_candid(target, "fixture_progress", ()).unwrap();
    assert!(matches!(empty.unwrap(), Err(ImportError::NotBegun)));
    assert!(first_row(&fixture, target).is_empty());
    assert_application_gate(pic, target, false);
    grant(
        pic,
        store,
        fixture.root,
        FixtureGrantRequest {
            binding: original.assignment.grant.binding.clone(),
            expected_revision: 1,
            enabled: false,
        },
    )
    .unwrap();
    grant(
        pic,
        store,
        fixture.root,
        FixtureGrantRequest {
            binding: replacement.assignment.grant.binding.clone(),
            expected_revision: 2,
            enabled: true,
        },
    )
    .unwrap();
    configure_runtime(pic, target, fixture.root, replacement.directory);
    wait_for_canic_install_callback(pic, target);
    drive_icydb_startup(pic, target);
    let FixtureProvisioningStatus::Complete(receipt) = wait_complete(pic, target) else {
        panic!("replacement completes after held old reply");
    };
    assert_eq!(receipt.binding, replacement.assignment.grant.binding);
    assert_eq!(receipt.completion_summary, descriptor.completion_summary);
    assert_eq!(
        crate::fixture_provisioning::store::pull(
            pic,
            target,
            store,
            FixtureChunkRead {
                grant: original.assignment.grant,
                index: 0
            }
        ),
        Err(FixtureStoreError::Authority)
    );
}

fn arm_chunk_reply(fixture: &CanicIcydbLifecycleFixture, store: Principal) {
    let pic = &fixture.pic;
    assert!(
        pic.update_candid_as::<bool, _>(
            store,
            Principal::from_slice(&[0x72; 29]),
            "canic_test_fixture_reply_arm",
            (HeldReply::Chunk,),
        )
        .is_err()
    );
    let armed: bool = pic
        .update_candid_as(
            store,
            fixture.root,
            "canic_test_fixture_reply_arm",
            (HeldReply::Chunk,),
        )
        .unwrap();
    assert!(armed);
}
