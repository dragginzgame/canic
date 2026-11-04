//! Installed API rechecks application policy after real randomness and Root calls.
use super::*;
use canic::dto::portable_issuance::{
    PortableCredentialIssuanceRefusal as Refusal, PortableCredentialIssuanceStatus as Status,
};

#[test]
fn permission_changes_after_await_preserve_intent_without_issuing() {
    let fixture = setup_fresh_active_component_registry();
    enroll_portable_fixture(&fixture);
    qualify(&fixture, 45, Refusal::Denied);
    qualify(&fixture, 46, Refusal::Conflict);
    qualify_retry(&fixture);
}

fn qualify_retry(fixture: &ActiveComponentRegistryFixture) {
    let account = Principal::from_slice(&[47]);
    let request = portable_issue_request(fixture, 47);
    assert_eq!(
        issue_fixture(fixture, request.clone(), account),
        Err(Refusal::Denied)
    );
    let before_retry = fixture
        .pic()
        .get_time()
        .unwrap()
        .as_nanos_since_unix_epoch();
    fixture.pic().advance_time(Duration::from_secs(1)).unwrap();
    let issued = issue_fixture(fixture, request.clone(), account).unwrap();
    let credential: ic_auth_protocol_types::PortableCredential =
        issued.credential.clone().try_into().unwrap();
    let claims = ic_auth::portable::decode_claims(&credential.claims).unwrap();
    assert!(claims.issued_at_ns <= before_retry);
    assert_eq!(
        issued.admission_deadline_ns,
        claims.issued_at_ns + 6_000_000_000
    );
    assert_eq!(
        reconcile_fixture(fixture, request.epoch, request.operation_id, account),
        Ok(Status::Issued(issued))
    );
}

fn qualify(fixture: &ActiveComponentRegistryFixture, id: u8, expected: Refusal) {
    let account = Principal::from_slice(&[id]);
    let request = portable_issue_request(fixture, id);
    assert_eq!(
        issue_fixture(fixture, request.clone(), account),
        Err(expected)
    );
    assert_eq!(
        reconcile_fixture(fixture, request.epoch, request.operation_id, account),
        Ok(Status::Unresolved)
    );
    fixture.pic().advance_time(Duration::from_secs(6)).unwrap();
    assert_eq!(
        issue_fixture(fixture, request.clone(), account),
        Err(Refusal::Expired)
    );
    assert_eq!(
        reconcile_fixture(fixture, request.epoch, request.operation_id, account),
        Ok(Status::Unresolved)
    );
    // The first policy check succeeded and its intent was persisted before awaits.
    // Reusing that exact operation for a different browser must conflict at begin.
    let mut changed = request.clone();
    changed.browser_public_key = portable_issue_request(fixture, id + 1).browser_public_key;
    assert_eq!(
        issue_fixture(fixture, changed, account),
        Err(Refusal::Conflict)
    );
    assert_eq!(
        reconcile_fixture(fixture, request.epoch, request.operation_id, account),
        Ok(Status::Unresolved)
    );
}
