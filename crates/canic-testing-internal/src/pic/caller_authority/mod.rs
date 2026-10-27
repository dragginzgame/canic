//! Module: pic::caller_authority
//!
//! Protected delivery loss/replay and native Wire qualification for managed fixtures.

use canic::{
    Error,
    dto::{caller_authority::*, role::OperationStatusRequest},
    testing::ManagedComponentGroupFixture,
};
use ic_platform::pocket_ic::PocketIc;
use ic_platform::principal::ExternalPrincipal as Principal;
use std::time::Duration;

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
    let request = canic_contracts::ops::caller_authority_status::publication_request(request);
    let message = pic
        .submit_call(
            hub,
            root,
            request.method,
            ic_wire::native::encode_request(
                request.method_id,
                &request.publication,
                canic::protocol::caller_authority::REQUEST_LIMITS,
            )
            .unwrap(),
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
    let request = canic_contracts::ops::caller_authority_status::publication_request(request);
    let response: Result<canic::dto::caller_authority_status::CallerAuthorityReceipt, Error> =
        ic_testkit::pic::update_wire(
            pic,
            hub,
            caller,
            ic_testkit::pic::WireCallSpec {
                method: request.method,
                method_id: request.method_id,
                request_limits: canic::protocol::caller_authority::REQUEST_LIMITS,
                response_limits: canic::protocol::caller_authority::RESPONSE_LIMITS,
            },
            &request.publication,
        )
        .unwrap();
    response.map(Into::into)
}

fn status(
    pic: &PocketIc,
    root: Principal,
    target: Principal,
    operation_id: [u8; 32],
) -> CallerAuthorityStatus {
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
            &OperationStatusRequest { operation_id },
        )
        .expect("caller authority Wire transport");
    response.expect("Root caller authority status").into()
}

fn probe(pic: &PocketIc, hub: Principal, shard: Principal) -> Result<Principal, Error> {
    ic_testkit::pic::update_wire::<_, Result<ic_platform::Principal, Error>>(
        pic,
        hub,
        shard,
        crate::pic::application_probe::CALLER_PROBE,
        &(),
    )
    .map(|reply| reply.map(Into::into))
    .unwrap()
}

const fn inactive() -> Result<Principal, Error> {
    Err(Error::from_registered(
        canic::diagnostics::codes::AUTHORITY_INACTIVE,
    ))
}
