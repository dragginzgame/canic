//! Full operator-to-Root-to-receiver observations with independent guard checks.
use super::*;
use canic::dto::{
    portable_inspection::{Page, PolicyReport, Refusal, RuntimeState, SessionObservation},
    root_observation::Request,
};

pub fn qualify(
    fixture: &ActiveComponentRegistryFixture,
    expected: &[SessionObservation],
    browser: Principal,
) {
    qualify_sessions(fixture, expected, browser);
    qualify_policy(fixture, expected, browser);
}

fn qualify_sessions(
    fixture: &ActiveComponentRegistryFixture,
    expected: &[SessionObservation],
    browser: Principal,
) {
    use canic::protocol::root_observation::portable_sessions as sessions;
    let pic = fixture.pic();
    let target = fixture.verifier.canister_id;
    let sessions_spec = portable_spec(
        sessions::METHOD,
        sessions::METHOD_ID,
        sessions::REQUEST_LIMITS,
        sessions::RESPONSE_LIMITS,
    );
    let request = Request {
        canister_id: target.into(),
        input: canic::dto::portable_inspection::Request {
            after: None,
            limit: 32,
        },
    };
    let reply: Result<Result<Page, Refusal>, Error> = ic_testkit::pic::update_wire(
        pic,
        fixture.root,
        Principal::anonymous(),
        sessions_spec,
        &request,
    )
    .unwrap();
    let observed = reply.unwrap().unwrap();
    assert!(expected.iter().all(|row| observed.sessions.contains(row)));
    let denied: Result<Result<Page, Refusal>, Error> =
        ic_testkit::pic::update_wire(pic, fixture.root, browser, sessions_spec, &request).unwrap();
    assert!(denied.is_err());
    let bad_page = Request {
        canister_id: target.into(),
        input: canic::dto::portable_inspection::Request {
            after: None,
            limit: 0,
        },
    };
    let refused: Result<Result<Page, Refusal>, Error> = ic_testkit::pic::update_wire(
        pic,
        fixture.root,
        Principal::anonymous(),
        sessions_spec,
        &bad_page,
    )
    .unwrap();
    assert_eq!(refused.unwrap(), Err(Refusal::InvalidPage));
}

fn qualify_policy(
    fixture: &ActiveComponentRegistryFixture,
    expected: &[SessionObservation],
    browser: Principal,
) {
    use canic::protocol::root_observation::portable_policy as policy;
    let pic = fixture.pic();
    let target = fixture.verifier.canister_id;
    let policy_spec = portable_spec(
        policy::METHOD,
        policy::METHOD_ID,
        policy::REQUEST_LIMITS,
        policy::RESPONSE_LIMITS,
    );
    let request = Request {
        canister_id: target.into(),
        input: (),
    };
    let report: Result<Result<PolicyReport, Refusal>, Error> = ic_testkit::pic::update_wire(
        pic,
        fixture.root,
        Principal::anonymous(),
        policy_spec,
        &request,
    )
    .unwrap();
    let report = report.unwrap().unwrap();
    let declared = report.declared.unwrap();
    assert!(declared.receiver.is_some());
    assert!(
        expected
            .iter()
            .flat_map(|session| &session.scopes)
            .all(|scope| declared.allowed_scopes.contains(scope))
    );
    assert!(declared.default_session_ttl_secs <= declared.maximum_session_ttl_secs);
    let RuntimeState::Ready(enrollment) = report.runtime else {
        panic!("fresh controlled receiver must report ready enrollment");
    };
    assert_eq!(enrollment.authority, fixture.root.into());
    assert!(enrollment.expires_at_ns > report.observed_at_ns);
    assert!(
        enrollment
            .issuers
            .iter()
            .any(|entry| entry.issuer == fixture.issuer.canister_id.into())
    );
    let direct = canic::protocol::portable_inspection::policy::METHOD;
    let direct_spec = portable_spec(
        direct,
        canic::protocol::portable_inspection::policy::METHOD_ID,
        canic::protocol::portable_inspection::policy::REQUEST_LIMITS,
        canic::protocol::portable_inspection::policy::RESPONSE_LIMITS,
    );
    assert!(
        ic_testkit::pic::update_wire::<_, Result<PolicyReport, Refusal>>(
            pic,
            target,
            browser,
            direct_spec,
            &()
        )
        .is_err()
    );
    let self_target = Request {
        canister_id: fixture.root.into(),
        input: (),
    };
    let refused: Result<Result<PolicyReport, Refusal>, Error> = ic_testkit::pic::update_wire(
        pic,
        fixture.root,
        Principal::anonymous(),
        policy_spec,
        &self_target,
    )
    .unwrap();
    assert!(refused.is_err());
}

/// Qualify Root access and deny unrelated callers on the receiver endpoint.
pub fn qualify_receiver(
    fixture: &ActiveComponentRegistryFixture,
    browser: Principal,
    subject: ic_platform::Principal,
) -> Vec<SessionObservation> {
    let pic = fixture.pic();
    let target = fixture.verifier.canister_id;
    let issuer = fixture.issuer.canister_id;
    let inspection = portable_spec(
        canic::protocol::portable_inspection::METHOD,
        canic::protocol::portable_inspection::METHOD_ID,
        canic::protocol::portable_inspection::REQUEST_LIMITS,
        canic::protocol::portable_inspection::RESPONSE_LIMITS,
    );
    let page = canic::dto::portable_inspection::Request {
        after: None,
        limit: 1,
    };
    let observed: Result<
        canic::dto::portable_inspection::Page,
        canic::dto::portable_inspection::Refusal,
    > = ic_testkit::pic::update_wire(pic, target, fixture.root, inspection, &page).unwrap();
    let observed = observed.unwrap();
    assert_eq!(observed.sessions.len(), 1);
    assert_eq!(observed.sessions[0].caller, browser.into());
    assert_eq!(observed.sessions[0].subject, subject);
    assert!(!observed.sessions[0].expired);
    for caller in [browser, Principal::anonymous(), issuer] {
        assert!(
            ic_testkit::pic::update_wire::<
                _,
                Result<
                    canic::dto::portable_inspection::Page,
                    canic::dto::portable_inspection::Refusal,
                >,
            >(pic, target, caller, inspection, &page,)
            .is_err()
        );
    }

    observed.sessions
}
