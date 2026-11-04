//! Maximum canonical grant shapes and the retained journal's byte boundary.

use super::*;
use ic_auth_protocol_types::limits;

fn maximum_grants() -> Vec<PortableEnrollmentGrant> {
    (0..limits::MAX_GRANTS)
        .map(|role| PortableEnrollmentGrant {
            role: format!("role{role:02}{}", "x".repeat(limits::MAX_LABEL_BYTES - 6)),
            scopes: (0..limits::MAX_SCOPES_PER_GRANT)
                .map(|scope| format!("scope{scope:02}{}", "x".repeat(limits::MAX_LABEL_BYTES - 7)))
                .collect(),
        })
        .collect()
}

fn maximum_request(id: u8) -> IssuePortableCredential {
    let mut request = request(id);
    request.grants = maximum_grants();
    request
}

fn key_hash(key: &PortableIssuerKey) -> [u8; 32] {
    let PortableIssuerKey::Ed25519 { public_key } = key else {
        panic!("fixture uses Ed25519 keys");
    };
    Sha256::digest(public_key).into()
}

#[test]
fn maximum_canonical_grants_complete_and_reconcile_within_the_journal_byte_budget() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let entry = enroll_with_grants(maximum_grants());
    let PortableIssuancePreparation::Pending(pending) =
        begin(domain(), account(), maximum_request(1), policy(), NOW).unwrap()
    else {
        panic!("new maximum-grant request");
    };
    let issued = complete(&pending, snapshot(&pending, entry, 1));
    assert!(PortableIssuerState::read().bytes.len() <= keys::JOURNAL_LIMITS.bytes);
    assert_eq!(
        reconcile(&domain(), account(), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Issued(issued.clone()))
    );
    let credential: PortableCredential = issued.credential.try_into().unwrap();
    let claims = ic_auth::portable::decode_claims(&credential.claims).unwrap();
    assert_eq!(claims.grants.len(), limits::MAX_GRANTS);
    for grant in claims.grants {
        assert_eq!(grant.target.as_str().len(), limits::MAX_LABEL_BYTES);
        assert_eq!(grant.scopes.len(), limits::MAX_SCOPES_PER_GRANT);
        assert!(
            grant
                .scopes
                .iter()
                .all(|scope| scope.as_str().len() == limits::MAX_LABEL_BYTES)
        );
    }
}

#[test]
fn pending_result_reservations_refuse_before_overcommitting_the_byte_budget() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let entry = enroll_with_grants(maximum_grants());
    let PortableIssuancePreparation::Pending(pending) =
        begin(domain(), account(), maximum_request(1), policy(), NOW).unwrap()
    else {
        panic!("new maximum-grant request");
    };
    let before = PortableIssuerState::read();
    assert!(matches!(
        begin(domain(), account(), maximum_request(2), policy(), NOW),
        Err(E::Capacity)
    ));
    assert_eq!(PortableIssuerState::read(), before);
    assert_eq!(load().unwrap().issuances.len(), 1);
    let result = complete(&pending, snapshot(&pending, entry, 1));
    assert_eq!(
        reconcile(&domain(), account(), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Issued(result))
    );
    assert_eq!(
        reconcile(&domain(), account(), 1, [2; 32]),
        Ok(PortableCredentialIssuanceStatus::Unresolved)
    );
}

#[test]
fn grant_bounds_refuse_without_creating_intent_or_changing_retained_work() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    enroll_with_grants(maximum_grants());
    let before = load().unwrap();
    let original = maximum_request(1);
    let mut extra_role = original.clone();
    extra_role.grants.push(PortableEnrollmentGrant {
        role: format!("role16{}", "x".repeat(limits::MAX_LABEL_BYTES - 6)),
        scopes: vec!["read".into()],
    });
    let mut extra_scope = original.clone();
    extra_scope.grants[0].scopes.push(format!(
        "scope32{}",
        "x".repeat(limits::MAX_LABEL_BYTES - 7)
    ));
    let mut long_role = original.clone();
    long_role.grants[0].role.push('x');
    let mut long_scope = original;
    long_scope.grants[0].scopes[0].push('x');
    for (request, expected) in [
        (extra_role, E::Capacity),
        (extra_scope, E::Capacity),
        (long_role, E::Invalid),
        (long_scope, E::Invalid),
    ] {
        assert!(
            matches!(begin(domain(), account(), request, policy(), NOW), Err(actual) if actual == expected)
        );
        assert_eq!(load().unwrap(), before);
    }
}

#[test]
fn unrecordable_root_retirement_drops_signing_authority_before_an_older_reply_can_finish() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let entry = enroll_with_grants(maximum_grants());
    let PortableIssuancePreparation::Pending(first) =
        begin(domain(), account(), maximum_request(1), policy(), NOW).unwrap()
    else {
        panic!("new maximum-grant request");
    };
    let original = complete(&first, snapshot(&first, entry.clone(), 1));
    let mut small = request(2);
    small.grants = vec![PortableEnrollmentGrant {
        role: entry.grants[0].role.clone(),
        scopes: vec![entry.grants[0].scopes[0].clone()],
    }];
    let PortableIssuancePreparation::Pending(pending) =
        begin(domain(), account(), small, policy(), NOW).unwrap()
    else {
        panic!("new bounded request");
    };
    let older = snapshot(&pending, entry.clone(), 2);
    let mut revoked = older.clone();
    revoked.revision = 3;
    revoked.generation = 2;
    revoked.retired_keys = vec![key_hash(&entry.key)];
    revoked.entries = (10..12)
        .map(|id| {
            let mut other = entry.clone();
            other.issuer = Principal::from_slice(&[id]);
            other.key = PortableIssuerKey::Ed25519 {
                public_key: SigningKey::from_bytes(&[id; 32]).verifying_key().to_bytes(),
            };
            other
        })
        .collect();
    revoked.entries.sort_by_key(|entry| key_hash(&entry.key));
    let reply: Result<_, crate::dto::error::Error> = Ok(revoked.clone());
    ic_wire::native::encode_response(
        portable_enrollment::METHOD_ID,
        &reply,
        portable_enrollment::RESPONSE_LIMITS,
    )
    .unwrap();
    let before = PortableIssuerState::read();
    assert!(KEY.with_borrow(Option::is_some));
    assert_eq!(accept_authority(&pending, revoked, NOW), Err(E::Capacity));
    assert_eq!(PortableIssuerState::read(), before);
    assert!(KEY.with_borrow(Option::is_none));
    assert_eq!(accept_authority(&pending, older, NOW), Err(E::Unavailable));
    assert_eq!(finish(&pending, NOW), Err(E::Unavailable));
    assert_eq!(
        reconcile(&domain(), account(), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Issued(original))
    );
}
