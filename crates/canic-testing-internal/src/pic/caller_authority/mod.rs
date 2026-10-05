//! Module: pic::caller_authority
//!
//! Protected delivery loss/replay and generated Candid qualification for managed fixtures.

mod candid_contract;

use candid::{CandidType, Deserialize, Principal};
use canic::{
    Error,
    dto::{caller_authority::*, role::OperationStatusRequest},
    testing::ManagedComponentGroupFixture,
};
use ic_testkit::pic::{CandidCallExt, PocketIc};
use std::time::Duration;

#[derive(CandidType)]
enum Command {
    CallerAuthority(CallerAuthorityCommand),
}

#[derive(CandidType, Debug, Deserialize)]
enum Response {
    CallerAuthority(CallerAuthorityReceipt),
}

#[derive(CandidType)]
enum StatusRequest {
    CallerAuthority(OperationStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum StatusResponse {
    CallerAuthority(CallerAuthorityStatus),
}

pub(super) fn qualify_candid(workspace: &std::path::Path) {
    candid_contract::qualify(workspace);
}

pub(super) fn qualify_delivery(
    fixture: &ManagedComponentGroupFixture,
    hub: Principal,
    shard: Principal,
) {
    let pic = fixture.pic();
    let root = fixture.root();
    let before = status(pic, root, hub, [0; 32]);
    let source = status(pic, root, shard, [0; 32]).authority.receiver;
    assert_eq!(probe(pic, hub, shard), Ok(shard));
    let publication = canic_core::api::caller_authority::CallerAuthorityApi::publication(
        before.authority.clone(),
        [0x91; 32],
        before.generation,
        CallerAuthorityChange::Grant(source.clone()),
    )
    .unwrap();
    assert_eq!(
        command(
            pic,
            shard,
            hub,
            CallerAuthorityCommand::Prepare(publication.clone())
        ),
        Err(Error::from_registered(
            canic::diagnostics::codes::AUTHORITY_UNAVAILABLE
        ))
    );
    assert_eq!(status(pic, root, hub, [0; 32]), before);
    assert_foreign_root_refused(pic, root, hub, &before, source.clone());

    // Deliberately discard the successful ingress reply; recover from durable status only.
    discard_reply(
        pic,
        root,
        hub,
        CallerAuthorityCommand::Prepare(publication.clone()),
    );
    assert_phase(pic, root, hub, &publication, CallerAuthorityPhase::Prepared);
    assert_eq!(probe(pic, hub, shard), inactive());
    fixture
        .upgrade_same_release(hub, Duration::from_mins(5))
        .unwrap();
    assert_phase(pic, root, hub, &publication, CallerAuthorityPhase::Prepared);
    assert_eq!(probe(pic, hub, shard), inactive());
    discard_reply(
        pic,
        root,
        hub,
        CallerAuthorityCommand::Commit(publication.clone()),
    );
    fixture
        .upgrade_same_release(hub, Duration::from_mins(5))
        .unwrap();
    assert_phase(
        pic,
        root,
        hub,
        &publication,
        CallerAuthorityPhase::Committed,
    );
    assert_eq!(probe(pic, hub, shard), Ok(shard));
    let complete = command(
        pic,
        root,
        hub,
        CallerAuthorityCommand::Complete(publication.clone()),
    )
    .unwrap();
    assert_eq!(complete.phase, CallerAuthorityPhase::Complete);
    let completed = status(pic, root, hub, publication.operation_id);
    for replay in [
        CallerAuthorityCommand::Prepare(publication.clone()),
        CallerAuthorityCommand::Commit(publication.clone()),
        CallerAuthorityCommand::Complete(publication.clone()),
    ] {
        assert_eq!(command(pic, root, hub, replay).unwrap(), complete);
        assert_eq!(status(pic, root, hub, publication.operation_id), completed);
    }
    qualify_component_fence(fixture, hub, shard, source, completed, complete);
}

fn qualify_component_fence(
    fixture: &ManagedComponentGroupFixture,
    hub: Principal,
    shard: Principal,
    source: canic::ids::CallerInstallation,
    completed: CallerAuthorityStatus,
    complete: CallerAuthorityReceipt,
) {
    let pic = fixture.pic();
    let root = fixture.root();
    let denial = canic_core::api::caller_authority::CallerAuthorityApi::publication(
        completed.authority,
        [0x92; 32],
        completed.generation,
        CallerAuthorityChange::DenyComponent(canic::ids::CallerComponentInstallation {
            binding: source.component().clone(),
            install_id: source.component_install_id,
        }),
    )
    .unwrap();
    discard_reply(
        pic,
        root,
        hub,
        CallerAuthorityCommand::Prepare(denial.clone()),
    );
    assert_eq!(probe(pic, hub, shard), inactive());
    command(
        pic,
        root,
        hub,
        CallerAuthorityCommand::Commit(denial.clone()),
    )
    .unwrap();
    command(
        pic,
        root,
        hub,
        CallerAuthorityCommand::Complete(denial.clone()),
    )
    .unwrap();
    let fenced = status(pic, root, hub, denial.operation_id);
    assert_eq!(
        command(
            pic,
            root,
            hub,
            CallerAuthorityCommand::Prepare(complete.publication.clone())
        )
        .unwrap(),
        complete
    );
    assert_eq!(status(pic, root, hub, denial.operation_id), fenced);
    assert_eq!(
        probe(pic, hub, shard),
        Err(Error::from_registered(
            canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED
        ))
    );
}

fn assert_foreign_root_refused(
    pic: &PocketIc,
    root: Principal,
    hub: Principal,
    before: &CallerAuthorityStatus,
    mut source: canic::ids::CallerInstallation,
) {
    match &mut source.binding {
        canic::ids::ManagedCanisterBinding::Component(binding) => {
            binding.fleet_subnet_root = Principal::from_slice(&[91; 29]);
        }
        canic::ids::ManagedCanisterBinding::ComponentChild(binding) => {
            binding.component.fleet_subnet_root = Principal::from_slice(&[91; 29]);
        }
    }
    let foreign = canic_core::api::caller_authority::CallerAuthorityApi::publication(
        before.authority.clone(),
        [0x93; 32],
        before.generation,
        CallerAuthorityChange::Grant(source),
    )
    .unwrap();
    assert_eq!(
        command(pic, root, hub, CallerAuthorityCommand::Prepare(foreign)),
        Err(Error::from_registered(
            canic::diagnostics::codes::AUTHORITY_CONFLICT
        ))
    );
    assert_eq!(status(pic, root, hub, [0; 32]), *before);
}

fn discard_reply(pic: &PocketIc, root: Principal, hub: Principal, request: CallerAuthorityCommand) {
    let message = pic
        .submit_call(
            hub,
            root,
            canic::protocol::CANIC_COMMAND,
            candid::encode_args((Command::CallerAuthority(request),)).unwrap(),
        )
        .unwrap();
    for _ in 0..12 {
        pic.tick();
        if pic.ingress_status(message.clone()).is_some() {
            return;
        }
    }
    panic!("protected publication must reach a terminal transport outcome");
}

fn assert_phase(
    pic: &PocketIc,
    root: Principal,
    hub: Principal,
    publication: &CallerAuthorityPublication,
    phase: CallerAuthorityPhase,
) {
    assert_eq!(
        status(pic, root, hub, publication.operation_id).receipt,
        Some(CallerAuthorityReceipt {
            publication: publication.clone(),
            phase
        })
    );
}

fn command(
    pic: &PocketIc,
    caller: Principal,
    hub: Principal,
    request: CallerAuthorityCommand,
) -> Result<CallerAuthorityReceipt, Error> {
    let response: Result<Response, Error> = pic
        .update_candid_as(
            hub,
            caller,
            canic::protocol::CANIC_COMMAND,
            (Command::CallerAuthority(request),),
        )
        .unwrap();
    response.map(|Response::CallerAuthority(receipt)| receipt)
}

fn status(
    pic: &PocketIc,
    root: Principal,
    target: Principal,
    operation_id: [u8; 32],
) -> CallerAuthorityStatus {
    let response: Result<StatusResponse, Error> = pic
        .query_candid_as(
            target,
            root,
            canic::protocol::CANIC_CONTROL_STATUS,
            (StatusRequest::CallerAuthority(OperationStatusRequest {
                operation_id,
            }),),
        )
        .unwrap();
    let StatusResponse::CallerAuthority(status) = response.unwrap();
    status
}

fn probe(pic: &PocketIc, hub: Principal, shard: Principal) -> Result<Principal, Error> {
    pic.update_candid_as(hub, shard, "test_caller_probe", ())
        .unwrap()
}

const fn inactive() -> Result<Principal, Error> {
    Err(Error::from_registered(
        canic::diagnostics::codes::AUTHORITY_INACTIVE,
    ))
}
