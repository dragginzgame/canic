//! Installed Auth authority remains fenced after restoring a sealed Root snapshot.
use super::*;
use canic::{
    dto::{
        authority_restore::{
            AuthorityRestoreFencePhase, AuthorityRestoreFenceStatusResponse,
            AuthoritySnapshotRequest,
        },
        portable_enrollment::{
            PortableIssuerChallenge, PortableIssuerRegistrationRefusal, RetirePortableIssuer,
        },
        portable_issuance::{PortableCredentialIssuanceRefusal, PortableCredentialIssuanceStatus},
    },
    protocol::{portable_enrollment::registration, protected_state},
};

#[test]
fn sealed_root_snapshot_cannot_reopen_issuer_authority() {
    let fixture = setup_fresh_active_component_registry();
    let registration_request = enroll_portable_fixture(&fixture);
    let pic = fixture.pic();
    let account = Principal::from_slice(&[43]);
    issue_fixture(&fixture, portable_issue_request(&fixture, 90), account).unwrap();
    let request = AuthoritySnapshotRequest {
        operation_id: [91; 32],
    };
    let prepare = portable_spec(
        protected_state::snapshot_prepare::METHOD,
        protected_state::snapshot_prepare::METHOD_ID,
        protected_state::snapshot_prepare::REQUEST_LIMITS,
        protected_state::snapshot_prepare::RESPONSE_LIMITS,
    );
    let sealed: Result<AuthorityRestoreFenceStatusResponse, Error> =
        ic_testkit::pic::update_wire(pic, fixture.root, Principal::anonymous(), prepare, &request)
            .unwrap();
    assert_eq!(sealed.unwrap().phase, AuthorityRestoreFencePhase::Sealed);
    pic.stop_canister(fixture.root, None).unwrap();
    let snapshot = pic
        .take_canister_snapshot(fixture.root, None, None)
        .unwrap();
    pic.start_canister(fixture.root, None).unwrap();

    let resume = portable_spec(
        protected_state::snapshot_resume::METHOD,
        protected_state::snapshot_resume::METHOD_ID,
        protected_state::snapshot_resume::REQUEST_LIMITS,
        protected_state::snapshot_resume::RESPONSE_LIMITS,
    );
    let resumed: Result<AuthorityRestoreFenceStatusResponse, Error> =
        ic_testkit::pic::update_wire(pic, fixture.root, Principal::anonymous(), resume, &request)
            .unwrap();
    assert_eq!(resumed.unwrap().phase, AuthorityRestoreFencePhase::Open);
    retire(&fixture);
    pic.stop_canister(fixture.root, None).unwrap();
    pic.load_canister_snapshot(fixture.root, None, snapshot.id)
        .unwrap();
    pic.start_canister(fixture.root, None).unwrap();
    let resumed: Result<AuthorityRestoreFenceStatusResponse, Error> =
        ic_testkit::pic::update_wire(pic, fixture.root, Principal::anonymous(), resume, &request)
            .unwrap();
    assert_eq!(
        resumed.unwrap_err().code(),
        canic_contracts::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
    );
    let registration: Result<
        Result<PortableIssuerChallenge, PortableIssuerRegistrationRefusal>,
        _,
    > = ic_testkit::pic::update_wire(
        pic,
        fixture.root,
        Principal::anonymous(),
        registration_spec(
            registration::prepare::METHOD,
            registration::prepare::METHOD_ID,
        ),
        &registration_request,
    );
    assert!(registration.is_err());

    let issue = portable_issue_request(&fixture, 91);
    assert_eq!(
        issue_fixture(&fixture, issue.clone(), account),
        Err(PortableCredentialIssuanceRefusal::Transport)
    );
    assert_eq!(
        reconcile_fixture(&fixture, issue.epoch, issue.operation_id, account),
        Ok(PortableCredentialIssuanceStatus::Unresolved)
    );
    drop(fixture);
}

#[test]
fn restored_issuer_heap_cannot_override_roots_retirement() {
    let fixture = setup_fresh_active_component_registry();
    enroll_portable_fixture(&fixture);
    let pic = fixture.pic();
    let issuer = fixture.issuer.canister_id;
    let account = Principal::from_slice(&[43]);
    issue_fixture(&fixture, portable_issue_request(&fixture, 92), account).unwrap();
    let request = portable_issue_request(&fixture, 93);
    pic.stop_canister(issuer, Some(fixture.root)).unwrap();
    let snapshot = pic
        .take_canister_snapshot(issuer, Some(fixture.root), None)
        .unwrap();
    retire(&fixture);
    pic.load_canister_snapshot(issuer, Some(fixture.root), snapshot.id)
        .unwrap();
    pic.start_canister(issuer, Some(fixture.root)).unwrap();
    assert_eq!(
        issue_fixture(&fixture, request.clone(), account),
        Err(PortableCredentialIssuanceRefusal::Unavailable)
    );
    assert_eq!(
        reconcile_fixture(&fixture, request.epoch, request.operation_id, account),
        Ok(PortableCredentialIssuanceStatus::Unresolved)
    );
    drop(fixture);
}

fn retire(fixture: &ActiveComponentRegistryFixture) {
    let result: Result<(), PortableIssuerRegistrationRefusal> = ic_testkit::pic::update_wire(
        fixture.pic(),
        fixture.root,
        Principal::anonymous(),
        registration_spec(
            registration::retire::METHOD,
            registration::retire::METHOD_ID,
        ),
        &RetirePortableIssuer {
            issuer: fixture.issuer.canister_id.into(),
        },
    )
    .unwrap();
    result.unwrap();
}
