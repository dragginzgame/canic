//! Canonical grant limits and byte-capacity recovery through installed endpoints.

use super::*;
use canic::dto::{
    portable_enrollment::{
        PortableEnrollmentGrant, PortableIssuerCustody, PreparePortableIssuerKey,
    },
    portable_issuance::{
        IssuePortableCredential, PortableCredentialIssuanceRefusal as Refusal,
        PortableCredentialIssuanceStatus as Status,
    },
};
use ic_auth_protocol_types::limits;

const MAX_WIDE_ADMISSION_INSTRUCTIONS: u64 = 600_000_000;
const MAX_WIDE_RETAINED_INSTRUCTIONS: u64 = 300_000_000;
const APPLICATION_SCOPE: &str = "app00:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";

fn maximum_grants() -> Vec<PortableEnrollmentGrant> {
    (0..limits::MAX_GRANTS)
        .map(|role| PortableEnrollmentGrant {
            // The installed receiver's actual role occupies the first slot.
            role: if role == 0 {
                "issuer".into()
            } else {
                format!("wide{role:02}{}", "x".repeat(limits::MAX_LABEL_BYTES - 6))
            },
            scopes: std::iter::once(APPLICATION_SCOPE.into())
                .chain((1..limits::MAX_SCOPES_PER_GRANT).map(|scope| {
                    format!("wide{scope:02}{}", "x".repeat(limits::MAX_LABEL_BYTES - 6))
                }))
                .collect(),
        })
        .collect()
}

#[test]
fn maximum_grant_issuance_recovers_and_byte_refusal_preserves_the_original_login() {
    let fixture = setup_fresh_active_component_registry();
    let prepared = prepare_volatile_key(
        &fixture,
        &PreparePortableIssuerKey {
            operation_id: [21; 32],
            custody: PortableIssuerCustody::TrustedVolatileLocal,
        },
    )
    .unwrap();
    let grants = maximum_grants();
    register_volatile_key_with_grants(&fixture, &prepared, Some(grants.clone()));
    let account = Principal::from_slice(&[44]);
    let mut request = portable_issue_request(&fixture, 5);
    request.grants = grants;
    assert_endpoint_bounds(&fixture, &request, account);
    let original = issuer_capacity::measure_with_ceiling(
        &fixture,
        "maximum-grants-issuance",
        "test_issue_portable_credential",
        MAX_WIDE_ADMISSION_INSTRUCTIONS,
        || issue_fixture(&fixture, request.clone(), account),
    )
    .unwrap();
    let retry = measure(
        &fixture,
        "maximum-grants-retry",
        "test_issue_portable_credential",
        || issue_fixture(&fixture, request.clone(), account),
    );
    assert_eq!(retry, Ok(original.clone()));
    let lookup = measure(
        &fixture,
        "maximum-grants-lookup",
        "test_reconcile_portable_credential",
        || reconcile_fixture(&fixture, request.epoch, request.operation_id, account),
    );
    assert_eq!(lookup, Ok(Status::Issued(original.clone())));
    let mut overflow = request.clone();
    overflow.operation_id = [6; 32];
    let refused = measure(
        &fixture,
        "journal-byte-capacity",
        "test_issue_portable_credential",
        || issue_fixture(&fixture, overflow.clone(), account),
    );
    assert_eq!(refused, Err(Refusal::Capacity));
    assert_eq!(
        reconcile_fixture(&fixture, overflow.epoch, overflow.operation_id, account),
        Ok(Status::Unresolved)
    );
    assert_eq!(
        reconcile_fixture(&fixture, request.epoch, request.operation_id, account),
        Ok(Status::Issued(original.clone()))
    );
    renew_after_cleanup(&fixture, request, original, account);
    drop(fixture);
}

fn assert_endpoint_bounds(
    fixture: &ActiveComponentRegistryFixture,
    original: &IssuePortableCredential,
    account: Principal,
) {
    let mut role_overflow = original.clone();
    role_overflow.grants.push(PortableEnrollmentGrant {
        role: "wide99".into(),
        scopes: vec![APPLICATION_SCOPE.into()],
    });
    let mut scope_overflow = original.clone();
    scope_overflow.grants[0].scopes.push("wide99".into());
    let mut label_overflow = original.clone();
    label_overflow.grants[0].scopes[1].push('x');
    for (request, expected) in [
        (role_overflow, Refusal::Capacity),
        (scope_overflow, Refusal::Capacity),
        (label_overflow, Refusal::Invalid),
    ] {
        assert_eq!(issue_fixture(fixture, request, account), Err(expected));
        assert_eq!(
            reconcile_fixture(fixture, original.epoch, original.operation_id, account),
            Ok(Status::Unresolved)
        );
    }
    // The wide namespace is an explicit account permission, not a Root grant alone.
    assert_eq!(
        issue_fixture(fixture, original.clone(), Principal::from_slice(&[43])),
        Err(Refusal::Denied)
    );
}

fn renew_after_cleanup(
    fixture: &ActiveComponentRegistryFixture,
    mut request: IssuePortableCredential,
    original: canic::dto::portable_issuance::IssuedPortableCredential,
    account: Principal,
) {
    fixture
        .pic()
        .advance_time(Duration::from_secs(71))
        .expect("PocketIC advance_time");
    fixture.pic().tick().expect("PocketIC tick");
    let maintenance = measure(
        fixture,
        "maximum-grants-pruning",
        "test_reclaim_portable_issuance",
        || issuer_retention::reclaim_as_root(fixture),
    );
    assert_eq!(maintenance.removed_issuances, 1);
    assert_eq!(maintenance.removed_logins, 0);
    assert_eq!(
        issue_fixture(fixture, request.clone(), account),
        Err(Refusal::EpochClosed)
    );
    request.epoch = maintenance.epoch;
    request.operation_id = [7; 32];
    let renewed = issuer_capacity::measure_with_ceiling(
        fixture,
        "maximum-grants-renewal",
        "test_issue_portable_credential",
        MAX_WIDE_ADMISSION_INSTRUCTIONS,
        || issue_fixture(fixture, request, account),
    )
    .unwrap();
    let original_claims = issuer_retention::claims(&original);
    let renewed_claims = issuer_retention::claims(&renewed);
    assert_eq!(renewed_claims.grants.len(), limits::MAX_GRANTS);
    assert_eq!(
        renewed_claims.login_started_at_ns,
        original_claims.login_started_at_ns
    );
    assert_eq!(
        renewed_claims.login_deadline_ns,
        original_claims.login_deadline_ns
    );
    issuer_retention::admit_credential(fixture, renewed, account);
}

fn measure<T>(
    fixture: &ActiveComponentRegistryFixture,
    label: &str,
    method: &str,
    action: impl FnOnce() -> T,
) -> T {
    issuer_capacity::measure_with_ceiling(
        fixture,
        label,
        method,
        MAX_WIDE_RETAINED_INSTRUCTIONS,
        action,
    )
}
