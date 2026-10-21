//! Installed participant proof driven by the canonical Root checkpoint journal.
//! The host fixture retains Root records outside snapshots; production Root storage,
//! discovery, RPC dispatch and paid snapshot orchestration remain separate obligations.

pub(super) mod unit;

use canic::{Error, dto::application_checkpoint::ApplicationCheckpointParticipantStatus};
use canic_contracts::{
    dto::{
        role::OperationStatusRequest,
        wire::projection::caller_authority::{StatusRequest, StatusResponse},
    },
    ids::{ApplicationCheckpointBinding as Binding, ApplicationCheckpointPhase as Phase},
};
use canic_core::shared_support::application_checkpoint_root::{
    RootCheckpointPhase, RootCheckpointRecord,
};
use ic_platform::{Principal, candid::contract::CandidType};
use ic_testkit::pic::{
    CanisterInstallExt, CanisterSnapshotTarget, PocketIc, PocketIcSnapshotExt,
    SnapshotRestoreFunding,
};
use serde::Deserialize;

/// Fixture-owned business and hook evidence returned by the participant.
#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
struct CheckpointView {
    retained: Option<ApplicationCheckpointParticipantStatus>,
    counter: u64,
    prepare_runs: u64,
    release_runs: u64,
}

pub(super) fn qualify(fixture: &super::ManagedLifecycleFixture, target: Principal) {
    let pic = &fixture.pic;
    pic.add_cycles(target, super::SNAPSHOT_RESTORE_MINIMUM_CYCLES);
    let root = fixture.root;
    let mut issued = issue(None, vec![authority(pic, target, root)], [1; 32]);
    let first = issued.participants()[0].binding().clone();
    let invalid = Error::from_registered(canic::diagnostics::codes::AUTHORITY_INVALID_STATE);
    assert_eq!(increment(pic, target), Ok(1));
    let unchanged = view(pic, target);
    assert!(
        transition(
            pic,
            target,
            Principal::anonymous(),
            &first,
            Phase::Preparing
        )
        .is_err()
    );
    let mut foreign = first.clone();
    foreign.authority.receiver.install_id = [9; 32];
    assert!(transition(pic, target, root, &foreign, Phase::Preparing).is_err());
    assert_eq!(view(pic, target), unchanged);

    // Discard the original reply; durable phase, not response retention, owns recovery.
    let _ = transition(pic, target, root, &first, Phase::Preparing).unwrap();
    assert_eq!(increment(pic, target), Err(invalid));
    pic.wait_out_install_code_rate_limit(std::time::Duration::from_mins(5));
    pic.upgrade_canister(target, fixture.wasm.clone(), super::upgrade_args(), None)
        .unwrap();
    assert_eq!(view(pic, target).retained.unwrap().phase, Phase::Preparing);
    assert_eq!(increment(pic, target), Err(invalid));
    assert_eq!(
        transition(pic, target, root, &first, Phase::Preparing),
        Ok(Phase::Preparing)
    );
    for _ in 0..2 {
        assert_eq!(
            transition(pic, target, root, &first, Phase::Prepared),
            Ok(Phase::Prepared)
        );
    }
    let prepared = view(pic, target);
    assert_eq!(prepared.prepare_runs, 1);
    // A missing Root acknowledgement must not infer preparation from the IC effect.
    assert_eq!(issued.phase(), RootCheckpointPhase::Preparing);
    issued = retained(&issued.acknowledge(&first, Phase::Prepared).unwrap());
    assert_eq!(issued.phase(), RootCheckpointPhase::Prepared);
    let snapshots = pic
        .capture_snapshots([CanisterSnapshotTarget {
            canister_id: target,
            sender: None,
        }])
        .unwrap();
    issued = retained(&issued.release());
    release(pic, target, root, &first);
    issued = retained(&issued.acknowledge(&first, Phase::Released).unwrap());
    assert_eq!(increment(pic, target), Ok(2));
    assert_eq!(
        transition(pic, target, root, &first, Phase::Preparing),
        Err(invalid)
    );

    // Releasing before a delayed prepare still leaves a durable high-water record.
    issued = issue(Some(&issued), vec![first.authority.clone()], [2; 32]);
    let second = issued.participants()[0].binding().clone();
    issued = retained(&issued.release());
    release(pic, target, root, &second);
    issued = retained(&issued.acknowledge(&second, Phase::Released).unwrap());
    pic.wait_out_install_code_rate_limit(std::time::Duration::from_mins(5));
    pic.upgrade_canister(target, fixture.wasm.clone(), super::upgrade_args(), None)
        .unwrap();
    assert_eq!(
        transition(pic, target, root, &second, Phase::Preparing),
        Err(invalid)
    );
    assert_eq!(view(pic, target).counter, 2);

    // The test driver holds source and fresh authority outside the loaded snapshot.
    pic.restore_snapshots(
        &snapshots,
        SnapshotRestoreFunding::TopUpTo {
            minimum_cycles: super::SNAPSHOT_RESTORE_MINIMUM_CYCLES,
        },
    )
    .unwrap();
    assert_eq!(view(pic, target), prepared);
    assert_eq!(increment(pic, target), Err(invalid));
    // Loading application memory cannot rewind the independently retained Root sequence.
    assert_eq!(issued.participants()[0].binding().sequence, 2);
    assert_eq!(issued.phase(), RootCheckpointPhase::Released);
    verify_restore_release(pic, target, root, first, prepared, issued);
}

// Exercise fresh restore authority and effect-free release replay after snapshot load.
fn verify_restore_release(
    pic: &PocketIc,
    target: Principal,
    root: Principal,
    first: Binding,
    prepared: CheckpointView,
    issued: RootCheckpointRecord,
) {
    let invalid = Error::from_registered(canic::diagnostics::codes::AUTHORITY_INVALID_STATE);
    let mut issued = issue(Some(&issued), vec![first.authority.clone()], [3; 32]);
    let fresh = issued.participants()[0].binding().clone();
    assert_eq!(fresh.sequence, 3);
    let mut wrong_source = first.clone();
    wrong_source.unit_digest = [8; 32];
    assert_eq!(
        restore(pic, target, root, &wrong_source, &fresh),
        Err(invalid)
    );
    assert_eq!(view(pic, target), prepared);
    for _ in 0..2 {
        assert_eq!(
            restore(pic, target, root, &first, &fresh),
            Ok(Phase::Preparing)
        );
    }
    assert_eq!(
        transition(pic, target, root, &fresh, Phase::Prepared),
        Ok(Phase::Prepared)
    );
    issued = retained(&issued.acknowledge(&fresh, Phase::Prepared).unwrap());
    issued = retained(&issued.release());
    release(pic, target, root, &fresh);
    let completed = view(pic, target);
    // Lose the release acknowledgement after the application commits it, then
    // recover the original Root journal and retry the exact issued binding.
    issued = retained(&issued);
    assert_eq!(issued.phase(), RootCheckpointPhase::Releasing);
    assert!(!issued.membership_mutation_allowed());
    assert_eq!(
        transition(pic, target, root, &fresh, Phase::Releasing),
        Ok(Phase::Released)
    );
    assert_eq!(
        transition(pic, target, root, &fresh, Phase::Released),
        Ok(Phase::Released)
    );
    assert_eq!(view(pic, target), completed);
    issued = retained(&issued.acknowledge(&fresh, Phase::Released).unwrap());
    assert_eq!(issued.phase(), RootCheckpointPhase::Released);
    assert!(issued.membership_mutation_allowed());
    assert_eq!(issued.release(), issued);
    assert_eq!(completed.prepare_runs, 2);
    assert_eq!(completed.release_runs, 1);
    assert_eq!(
        transition(pic, target, root, &first, Phase::Preparing),
        Err(invalid)
    );
    assert_eq!(increment(pic, target), Ok(2));
}

// The fixture round-trips the production Root model, rather than inventing fresh
// participant sequence numbers. This is host-held evidence, not a stable Root store.
fn issue(
    previous: Option<&RootCheckpointRecord>,
    authorities: Vec<canic_contracts::ids::CallerReceiverAuthority>,
    operation_id: [u8; 32],
) -> RootCheckpointRecord {
    let sequence = previous.map_or(0, |record| record.participants()[0].binding().sequence);
    let root = authorities[0].issuer.clone();
    retained(
        &RootCheckpointRecord::begin(
            previous,
            &root,
            sequence,
            operation_id,
            [7; 32],
            authorities,
        )
        .unwrap(),
    )
}

fn authority(
    pic: &PocketIc,
    target: Principal,
    root: Principal,
) -> canic_contracts::ids::CallerReceiverAuthority {
    let response: Result<StatusResponse, Error> = ic_testkit::pic::query_candid_args(
        pic,
        target,
        root,
        canic::protocol::CANIC_CONTROL_STATUS,
        (StatusRequest::CallerAuthority(OperationStatusRequest {
            operation_id: [0; 32],
        }),),
        canic::testing::CANDID_REPLY_LIMITS,
    )
    .map(|(reply,)| reply)
    .unwrap();
    let StatusResponse::CallerAuthority(status) = response.unwrap();
    status.authority
}

fn retained(record: &RootCheckpointRecord) -> RootCheckpointRecord {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(record, &mut bytes).unwrap();
    let recovered = ciborium::de::from_reader(bytes.as_slice()).unwrap();
    assert_eq!(record, &recovered);
    recovered
}

fn view(pic: &PocketIc, target: Principal) -> CheckpointView {
    ic_testkit::pic::query_candid_args::<(Result<CheckpointView, Error>,), _>(
        pic,
        target,
        Principal::anonymous(),
        "checkpoint_view",
        (),
        canic::testing::CANDID_REPLY_LIMITS,
    )
    .map(|(reply,)| reply)
    .unwrap()
    .unwrap()
}

fn increment(pic: &PocketIc, target: Principal) -> Result<u64, Error> {
    ic_testkit::pic::update_candid_args(
        pic,
        target,
        Principal::anonymous(),
        "checkpoint_increment",
        (),
        canic::testing::CANDID_REPLY_LIMITS,
    )
    .map(|(reply,)| reply)
    .unwrap()
}

fn transition(
    pic: &PocketIc,
    target: Principal,
    root: Principal,
    binding: &Binding,
    phase: Phase,
) -> Result<Phase, Error> {
    ic_testkit::pic::update_candid_args(
        pic,
        target,
        root,
        "checkpoint_transition",
        (binding, phase),
        canic::testing::CANDID_REPLY_LIMITS,
    )
    .map(|(reply,)| reply)
    .unwrap()
}

fn release(pic: &PocketIc, target: Principal, root: Principal, binding: &Binding) {
    assert_eq!(
        transition(pic, target, root, binding, Phase::Releasing),
        Ok(Phase::Releasing)
    );
    assert_eq!(
        transition(pic, target, root, binding, Phase::Released),
        Ok(Phase::Released)
    );
}

fn restore(
    pic: &PocketIc,
    target: Principal,
    root: Principal,
    captured: &Binding,
    fresh: &Binding,
) -> Result<Phase, Error> {
    ic_testkit::pic::update_candid_args(
        pic,
        target,
        root,
        "checkpoint_restore",
        (captured, fresh),
        canic::testing::CANDID_REPLY_LIMITS,
    )
    .map(|(reply,)| reply)
    .unwrap()
}
