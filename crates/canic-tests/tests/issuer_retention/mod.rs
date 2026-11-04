//! Installed issuer reclamation preserves login authority and fences forgotten work.

use super::*;
use canic::dto::{
    portable_enrollment::{PortableIssuerCustody, PreparePortableIssuerKey},
    portable_issuance::{
        PortableCredentialIssuanceRefusal as Refusal, PortableCredentialIssuanceStatus as Status,
        PortableIssuanceMaintenance,
    },
};

#[test]
fn issuer_history_reclamation_preserves_original_login_and_closes_pruned_requests() {
    let fixture = setup_fresh_active_component_registry();
    let prepared = prepare_volatile_key(
        &fixture,
        &PreparePortableIssuerKey {
            operation_id: [21; 32],
            custody: PortableIssuerCustody::TrustedVolatileLocal,
        },
    )
    .unwrap();
    register_volatile_key(&fixture, &prepared);
    let account = Principal::from_slice(&[43]);
    let original_request = portable_issue_request(&fixture, 5);
    let original = issue_fixture(&fixture, original_request.clone(), account).unwrap();
    let original_claims = claims(&original);
    fixture
        .pic()
        .advance_time(Duration::from_secs(71))
        .expect("PocketIC advance_time");
    fixture.pic().tick().expect("PocketIC tick");
    assert_eq!(
        reconcile_fixture(&fixture, original_request.epoch, [5; 32], account).unwrap(),
        Status::Issued(original)
    );
    assert_reclamation_denied(&fixture);
    let maintenance = reclaim_as_root(&fixture);
    assert!(maintenance.epoch > original_request.epoch);
    assert_eq!(maintenance.removed_issuances, 1);
    assert_eq!(maintenance.removed_logins, 0);
    assert_eq!(issuer_context(&fixture).epoch, maintenance.epoch);
    assert_eq!(
        reconcile_fixture(&fixture, original_request.epoch, [5; 32], account).unwrap(),
        Status::Unresolved
    );
    assert_eq!(
        issue_fixture(&fixture, original_request.clone(), account),
        Err(Refusal::EpochClosed)
    );
    let mut renewal = original_request;
    renewal.epoch = maintenance.epoch;
    renewal.operation_id = [6; 32];
    let renewed = issue_fixture(&fixture, renewal, account).unwrap();
    let renewed_claims = claims(&renewed);
    assert_eq!(
        renewed_claims.login_started_at_ns,
        original_claims.login_started_at_ns
    );
    assert_eq!(
        renewed_claims.login_deadline_ns,
        original_claims.login_deadline_ns
    );
    assert!(renewed_claims.issued_at_ns > original_claims.issued_at_ns);
    admit_credential(&fixture, renewed, account);
    drop(fixture);
}

const fn reclaim_spec() -> ic_testkit::pic::WireCallSpec {
    portable_spec(
        "test_reclaim_portable_issuance",
        0x5445_5354_0000_080f,
        canic::protocol::portable_issuance::REQUEST_LIMITS,
        canic::protocol::portable_issuance::RESPONSE_LIMITS,
    )
}

fn assert_reclamation_denied(fixture: &ActiveComponentRegistryFixture) {
    let denied: Result<Result<PortableIssuanceMaintenance, Refusal>, _> =
        ic_testkit::pic::update_wire(
            fixture.pic(),
            fixture.issuer.canister_id,
            portable_browser(5),
            reclaim_spec(),
            &(),
        );
    assert!(
        denied.is_err(),
        "only installed Root may reclaim issuer history"
    );
}

/// Invoke only the installed Root path, separately from the denial probe.
pub fn reclaim_as_root(fixture: &ActiveComponentRegistryFixture) -> PortableIssuanceMaintenance {
    let result: Result<PortableIssuanceMaintenance, Refusal> = ic_testkit::pic::update_wire(
        fixture.pic(),
        fixture.issuer.canister_id,
        fixture.root,
        reclaim_spec(),
        &(),
    )
    .unwrap();
    result.unwrap()
}

/// Read the upstream claims used to compare original login authority across tests.
pub fn claims(
    issued: &canic::dto::portable_issuance::IssuedPortableCredential,
) -> ic_auth_protocol_types::PortableClaims {
    let credential: ic_auth_protocol_types::PortableCredential =
        issued.credential.clone().try_into().unwrap();
    ic_auth::portable::decode_claims(&credential.claims).unwrap()
}

/// Admit a genuine credential for its bound browser and authenticated account.
pub fn admit_credential(
    fixture: &ActiveComponentRegistryFixture,
    renewed: canic::dto::portable_issuance::IssuedPortableCredential,
    account: Principal,
) {
    let browser = portable_browser(5);
    let result: Result<ic_auth_wire::SessionView, ic_auth_wire::model::AdmissionRefusal> =
        ic_testkit::pic::update_wire(
            fixture.pic(),
            fixture.verifier.canister_id,
            browser,
            portable_spec(
                "canic_establish_portable_session",
                ic_auth_wire::EstablishSession::ID,
                ic_auth_wire::EstablishSession::REQUEST_LIMITS,
                ic_auth_wire::EstablishSession::RESPONSE_LIMITS,
            ),
            &ic_auth_wire::SessionInput {
                credential: renewed.credential,
                scopes: ic_wire::List::new(vec![
                    ic_wire::Text::new(
                        "app00:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".into(),
                    )
                    .unwrap(),
                ])
                .unwrap(),
                ttl_secs: Some(1800),
            },
        )
        .unwrap();
    assert_eq!(result.unwrap().caller, browser.into());
    assert_eq!(
        portable_read(fixture.pic(), fixture.verifier.canister_id, browser)
            .unwrap()
            .subject,
        account.into()
    );
}
