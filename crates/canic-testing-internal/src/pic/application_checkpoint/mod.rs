//! Installed participant proof: durable business freeze, exact Root admission and restore.
//! Root dispatch qualification lives in `root`; paid snapshot orchestration is separate.

mod protocol;
mod root;

use canic::{
    Error, dto::application_checkpoint::ApplicationCheckpointCommand,
    protocol::application_checkpoint as checkpoint_protocol,
};
use canic_contracts::{
    dto::role::OperationStatusRequest,
    ids::{ApplicationCheckpointBinding as Binding, ApplicationCheckpointPhase as Phase},
};
use canic_core::control_plane_support::ops::application_checkpoint::command;
use ic_platform::principal::ExternalPrincipal as Principal;
use serde::Deserialize;
use {
    ic_platform::pocket_ic::PocketIc,
    ic_testkit::pic::{
        CanisterInstallExt, CanisterSnapshotTarget, PocketIcSnapshotExt, SnapshotRestoreFunding,
    },
};

pub(super) use root::qualify_original_host_workflow;
pub(super) use root::qualify_root_dispatch;

/// Fixture-owned business and hook evidence returned by the participant.
#[derive(Debug, Deserialize, Eq, PartialEq)]
struct CheckpointView {
    retained: Option<protocol::ParticipantStatus>,
    counter: u64,
    prepare_runs: u64,
    release_runs: u64,
}

pub(super) fn qualify(fixture: &super::ManagedLifecycleFixture, target: Principal) {
    let pic = &fixture.pic;
    pic.add_cycles(target, super::SNAPSHOT_RESTORE_MINIMUM_CYCLES);
    let root = fixture.root;
    let status = caller_authority_status(pic, target, root).unwrap();
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
    verify_restore_release(pic, target, root, first.clone(), prepared);
    qualify_wire_hook(pic, target, root, first);
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
    ic_testkit::pic::query_wire::<_, Result<CheckpointView, Error>>(
        pic,
        target,
        Principal::anonymous(),
        super::application_probe::CHECKPOINT_VIEW,
        &(),
    )
    .unwrap()
    .unwrap()
}

fn increment(pic: &PocketIc, target: Principal) -> Result<u64, Error> {
    ic_testkit::pic::update_wire(
        pic,
        target,
        Principal::anonymous(),
        super::application_probe::CHECKPOINT_INCREMENT,
        &(),
    )
    .unwrap()
}

fn transition(
    pic: &PocketIc,
    target: Principal,
    root: Principal,
    binding: &Binding,
    phase: Phase,
) -> Result<Phase, Error> {
    ic_testkit::pic::update_wire(
        pic,
        target,
        root,
        super::application_probe::CHECKPOINT_TRANSITION,
        &protocol::TransitionRequest {
            binding: binding.clone().into(),
            phase,
        },
    )
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
    ic_testkit::pic::update_wire(
        pic,
        target,
        root,
        super::application_probe::CHECKPOINT_RESTORE,
        &protocol::RestoreRequest {
            captured: captured.clone().into(),
            binding: fresh.clone().into(),
        },
    )
    .unwrap()
}

fn caller_authority_status(
    pic: &PocketIc,
    target: Principal,
    root: Principal,
) -> Result<canic::dto::caller_authority::CallerAuthorityStatus, Error> {
    let response: Result<canic::dto::caller_authority_status::CallerAuthorityStatus, Error> =
        ic_testkit::pic::query_wire(
            pic,
            target,
            root,
            ic_testkit::pic::WireCallSpec {
                method: canic::protocol::caller_authority_status::METHOD,
                method_id: canic::protocol::caller_authority_status::METHOD_ID,
                request_limits: canic::protocol::caller_authority_status::REQUEST_LIMITS,
                response_limits: canic::protocol::caller_authority_status::RESPONSE_LIMITS,
            },
            &OperationStatusRequest {
                operation_id: [0; 32],
            },
        )
        .expect("caller authority Wire transport");
    response.map(Into::into)
}

// Qualify the maintained hook codec and admission independently of Root scheduling.
fn qualify_wire_hook(pic: &PocketIc, target: Principal, root: Principal, mut selected: Binding) {
    selected.sequence = 4;
    selected.operation_id = [4; 32];
    let request = command(&selected, Phase::Preparing).unwrap();
    let before = view(pic, target);
    assert!(wire_hook(pic, target, Principal::anonymous(), &request).is_err());
    let mut foreign = request.clone();
    foreign.authority_digest[0] ^= 1;
    assert!(wire_hook(pic, target, root, &foreign).is_err());
    assert_eq!(view(pic, target), before);
    let expected = ApplicationCheckpointCommand {
        phase: Phase::Prepared,
        ..request
    };
    assert_eq!(wire_hook(pic, target, root, &request), Ok(expected.clone()));
    let prepared = view(pic, target);
    assert_eq!(prepared.prepare_runs, before.prepare_runs + 1);
    assert_eq!(wire_hook(pic, target, root, &request), Ok(expected));
    assert_eq!(view(pic, target), prepared);
    let release = ApplicationCheckpointCommand {
        phase: Phase::Releasing,
        ..request
    };
    let released = ApplicationCheckpointCommand {
        phase: Phase::Released,
        ..request
    };
    assert_eq!(wire_hook(pic, target, root, &release), Ok(released.clone()));
    let completed = view(pic, target);
    assert_eq!(completed.release_runs, before.release_runs + 1);
    assert_eq!(wire_hook(pic, target, root, &release), Ok(released));
    assert_eq!(
        wire_hook(pic, target, root, &request),
        Err(Error::from_registered(
            canic::diagnostics::codes::AUTHORITY_INVALID_STATE
        ))
    );
    assert_eq!(view(pic, target), completed);
}

fn wire_hook(
    pic: &PocketIc,
    target: Principal,
    sender: Principal,
    request: &canic::dto::application_checkpoint::ApplicationCheckpointCommand,
) -> Result<canic::dto::application_checkpoint::ApplicationCheckpointCommand, Error> {
    let bytes = ic_wire::native::encode_request(
        checkpoint_protocol::PARTICIPANT,
        request,
        checkpoint_protocol::LIMITS,
    )
    .unwrap();
    let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
        pic.update_call(
            target,
            sender,
            checkpoint_protocol::PARTICIPANT_METHOD,
            bytes,
        )
    })
    .unwrap()
    .unwrap();
    ic_wire::native::decode_response(
        checkpoint_protocol::PARTICIPANT,
        &reply,
        checkpoint_protocol::LIMITS,
    )
    .unwrap()
}
