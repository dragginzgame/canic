//! Installed participant proof: durable business freeze, exact Root admission and restore.
//! Root sequencing and paid snapshot orchestration remain separate product obligations.

use canic::{Error, dto::application_checkpoint::ApplicationCheckpointParticipantStatus};
use canic_contracts::{
    dto::{
        role::OperationStatusRequest,
        wire::projection::caller_authority::{StatusRequest, StatusResponse},
    },
    ids::{ApplicationCheckpointBinding as Binding, ApplicationCheckpointPhase as Phase},
};
use ic_platform::{candid::contract::CandidType, principal::ExternalPrincipal as Principal};
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
    let first = Binding {
        authority: status.authority,
        operation_id: [1; 32],
        sequence: 1,
        unit_digest: [7; 32],
    };
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
    let snapshots = pic
        .capture_snapshots([CanisterSnapshotTarget {
            canister_id: target,
            sender: None,
        }])
        .unwrap();
    release(pic, target, root, &first);
    assert_eq!(increment(pic, target), Ok(2));
    assert_eq!(
        transition(pic, target, root, &first, Phase::Preparing),
        Err(invalid)
    );

    // Releasing before a delayed prepare still leaves a durable high-water record.
    let mut second = first.clone();
    second.sequence = 2;
    second.operation_id = [2; 32];
    release(pic, target, root, &second);
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
    verify_restore_release(pic, target, root, first, prepared);
}

// Exercise fresh restore authority and effect-free release replay after snapshot load.
fn verify_restore_release(
    pic: &PocketIc,
    target: Principal,
    root: Principal,
    first: Binding,
    prepared: CheckpointView,
) {
    let invalid = Error::from_registered(canic::diagnostics::codes::AUTHORITY_INVALID_STATE);
    let mut fresh = first.clone();
    fresh.sequence = 3;
    fresh.operation_id = [3; 32];
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
    release(pic, target, root, &fresh);
    let completed = view(pic, target);
    assert_eq!(
        transition(pic, target, root, &fresh, Phase::Releasing),
        Ok(Phase::Released)
    );
    assert_eq!(
        transition(pic, target, root, &fresh, Phase::Released),
        Ok(Phase::Released)
    );
    assert_eq!(view(pic, target), completed);
    assert_eq!(completed.prepare_runs, 2);
    assert_eq!(completed.release_runs, 1);
    assert_eq!(
        transition(pic, target, root, &first, Phase::Preparing),
        Err(invalid)
    );
    assert_eq!(increment(pic, target), Ok(2));
}

fn view(pic: &PocketIc, target: Principal) -> CheckpointView {
    ic_testkit::pic::query_candid_args::<(Result<CheckpointView, Error>,), _>(
        pic,
        target,
        ic_platform::principal::ExternalPrincipal::anonymous(),
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
        ic_platform::principal::ExternalPrincipal::anonymous(),
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
