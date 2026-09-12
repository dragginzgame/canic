//! Interrupted automatic import and replacement grants against the actual retained Store.

use super::{
    ConsumerFault, assert_application_gate, drive, first_row, grant, progress, set_fault,
    wait_complete,
};
use crate::fixture_provisioning::{ImportError, ImportSnapshot, store};
use crate::{configure_runtime, drive_icydb_startup, wait_for_canic_install_callback};
use candid::{Principal, encode_one};
use canic::{
    Error,
    dto::fixture_provisioning::{
        FixtureChunkRead, FixtureDescriptor, FixtureGrant, FixtureGrantRequest,
        FixtureProvisioningStatus, FixtureStoreError,
    },
    protocol::{CANIC_WASM_STORE_CATALOG, CANIC_WASM_STORE_COMMAND},
};
use canic_control_plane::dto::template::{StoreCatalogRequest, StoreCatalogResponse, StoreCommand};
use canic_testing_internal::pic::{CanicIcydbLifecycleFixture, InstalledFixtureConsumer};
use ic_testkit::pic::{CandidCallExt, CanisterInstallExt, PocketIc};
use std::time::Duration;

pub(super) fn qualify(
    fixture: &CanicIcydbLifecycleFixture,
    store: Principal,
    controller: Principal,
    wasm: &[u8],
    descriptor: &FixtureDescriptor,
) {
    let pic = &fixture.pic;
    let original = fixture.install_fixture_consumer(store, descriptor.clone());
    let target = original.canister_id;
    grant(
        pic,
        store,
        fixture.root,
        request(&original.assignment.grant, 0, true),
    )
    .unwrap();
    set_fault(pic, target, ConsumerFault::PauseAfterFirstChunk);
    configure_runtime(pic, target, fixture.root, original.directory.clone());
    wait_for_canic_install_callback(pic, target);
    drive_icydb_startup(pic, target);
    drive(pic, 80);
    let partial = progress(fixture, target);
    assert_eq!(partial.next, 1);
    assert_eq!(
        partial.binding.installation,
        original.assignment.grant.binding.installation
    );
    assert!(partial.receipt.is_none());
    assert_eq!(first_row(fixture, target), vec![(0, 10)]);
    assert_application_gate(pic, target, false);

    pic.stop_canister(store, Some(controller)).unwrap();
    pic.wait_out_install_code_rate_limit(Duration::from_mins(5));
    let replacement = fixture.reinstall_fixture_consumer(&original, [0xa7; 32], 3);
    let empty: Result<Result<ImportSnapshot, ImportError>, Error> =
        pic.query_candid(target, "fixture_progress", ()).unwrap();
    assert!(matches!(empty.unwrap(), Err(ImportError::NotBegun)));
    pic.start_canister(store, Some(controller)).unwrap();
    replace_grant_after_lost_replies(fixture, store, controller, wasm, &original, &replacement);

    // Keep the replacement Prepared until its exact grant is retained. A temporary
    // Store outage then proves the new runtime starts from an empty application.
    pic.stop_canister(store, Some(controller)).unwrap();
    configure_runtime(pic, target, fixture.root, replacement.directory.clone());
    wait_for_canic_install_callback(pic, target);
    drive_icydb_startup(pic, target);
    drive(pic, 40);
    let fresh = progress(fixture, target);
    assert_eq!(
        fresh.binding.installation,
        replacement.assignment.grant.binding.installation
    );
    assert_eq!(fresh.next, 0);
    assert!(fresh.receipt.is_none());
    assert!(first_row(fixture, target).is_empty());
    assert_application_gate(pic, target, false);
    pic.start_canister(store, Some(controller)).unwrap();
    let complete = wait_complete(pic, target);
    let FixtureProvisioningStatus::Complete(receipt) = &complete else {
        unreachable!()
    };
    assert_eq!(receipt.binding, replacement.assignment.grant.binding);
    assert_eq!(receipt.completion_summary, descriptor.completion_summary);
    assert_eq!(
        progress(fixture, target).next,
        u64::try_from(descriptor.chunks.len()).unwrap()
    );
    assert_application_gate(pic, target, true);
    assert_eq!(
        store::pull(
            pic,
            target,
            store,
            FixtureChunkRead {
                grant: original.assignment.grant,
                index: 0,
            }
        ),
        Err(FixtureStoreError::Authority)
    );
    crate::fixture_provisioning::restart(fixture, target);
    drive(pic, 20);
    assert_eq!(super::status(pic, target), complete);
}

fn replace_grant_after_lost_replies(
    fixture: &CanicIcydbLifecycleFixture,
    store: Principal,
    controller: Principal,
    wasm: &[u8],
    original: &InstalledFixtureConsumer,
    replacement: &InstalledFixtureConsumer,
) {
    let pic = &fixture.pic;
    let root = fixture.root;
    let revoke = request(&original.assignment.grant, 1, false);
    discard_grant_reply(pic, store, root, revoke.clone());
    let revoked = observe_grant(pic, store, root, original.canister_id);
    assert_eq!(revoked.revision, 2);
    assert!(!revoked.enabled);
    assert_eq!(revoked.binding, original.assignment.grant.binding);
    assert_eq!(grant(pic, store, root, revoke.clone()).unwrap(), revoked);
    let issue = request(&replacement.assignment.grant, 2, true);
    discard_grant_reply(pic, store, root, issue.clone());
    store::restart(pic, store, controller, wasm);
    assert_eq!(
        observe_grant(pic, store, root, replacement.canister_id),
        replacement.assignment.grant
    );
    assert_eq!(
        grant(pic, store, root, issue).unwrap(),
        replacement.assignment.grant
    );
    assert_eq!(
        grant(pic, store, root, revoke),
        Err(FixtureStoreError::Conflict)
    );
    assert_eq!(
        grant(
            pic,
            store,
            root,
            request(&original.assignment.grant, 0, true)
        ),
        Err(FixtureStoreError::Conflict)
    );
    assert_eq!(
        observe_grant(pic, store, root, replacement.canister_id),
        replacement.assignment.grant
    );
}

fn request(grant: &FixtureGrant, expected_revision: u64, enabled: bool) -> FixtureGrantRequest {
    FixtureGrantRequest {
        expected_revision,
        binding: grant.binding.clone(),
        enabled,
    }
}

fn discard_grant_reply(
    pic: &PocketIc,
    store: Principal,
    root: Principal,
    request: FixtureGrantRequest,
) {
    let message = pic
        .submit_call(
            store,
            root,
            CANIC_WASM_STORE_COMMAND,
            encode_one(StoreCommand::SetFixtureGrant(Box::new(request))).unwrap(),
        )
        .unwrap();
    // Drain transport without decoding or retaining the command receipt. Recovery
    // must use the real Store's persisted grant and exact request identity.
    drop(pic.await_call(message).unwrap());
}

fn observe_grant(
    pic: &PocketIc,
    store: Principal,
    root: Principal,
    target: Principal,
) -> FixtureGrant {
    let response: Result<StoreCatalogResponse, Error> = pic
        .query_candid_as(
            store,
            root,
            CANIC_WASM_STORE_CATALOG,
            (StoreCatalogRequest::FixtureGrant(target),),
        )
        .unwrap();
    let StoreCatalogResponse::FixtureGrant(Some(grant)) = response.unwrap() else {
        panic!("expected retained target grant");
    };
    *grant
}
