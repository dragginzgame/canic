//! Reclamation safety under closed epochs, live logins and delayed completions.

use super::*;

const REMOVE_AT: u64 = NOW + 70 * SECOND;

fn request_in(id: u8, epoch: u64) -> IssuePortableCredential {
    IssuePortableCredential {
        epoch,
        ..request(id)
    }
}

fn start(
    request: IssuePortableCredential,
    limits: PortableIssuancePolicy,
    now_ns: u64,
) -> PortableIssuanceTicket {
    match begin(domain(), account(), request, limits, now_ns).unwrap() {
        PortableIssuancePreparation::Pending(pending) => pending,
        PortableIssuancePreparation::Ready(_) => panic!("expected a fresh retained operation"),
    }
}

fn root_at(
    pending: &PortableIssuanceTicket,
    entry: PortableIssuerEnrollment,
    revision: u64,
    now_ns: u64,
) -> PortableEnrollmentSnapshot {
    PortableEnrollmentSnapshot {
        authority: domain().authority,
        receiver: domain().issuer,
        fleet: domain().fleet,
        boot_challenge: challenge(pending, [revision.try_into().unwrap(); 32], now_ns).unwrap(),
        revision,
        generation: 1,
        issued_at_ns: now_ns,
        expires_at_ns: now_ns + 3_600 * SECOND,
        entries: vec![entry],
        retired_keys: vec![],
    }
}

#[test]
fn pruning_closes_old_operations_and_keeps_lookup_and_late_callbacks_effect_free() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let entry = enroll();
    let pending = pending_at(1, NOW);
    let original = complete(&pending, snapshot(&pending, entry.clone(), 1));
    let before = PortableIssuerState::read();
    assert_eq!(context(&domain()).unwrap().epoch, 1);
    assert_eq!(PortableIssuerState::read(), before);
    let early = reclaim(&domain(), REMOVE_AT - 1).unwrap();
    assert_eq!(
        (early.epoch, early.removed_issuances, early.removed_logins),
        (1, 0, 0)
    );
    assert_eq!(
        reconcile(&domain(), account(), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Issued(original))
    );
    let removed = reclaim(&domain(), REMOVE_AT).unwrap();
    assert_eq!(
        (
            removed.epoch,
            removed.removed_issuances,
            removed.removed_logins
        ),
        (2, 1, 0)
    );
    let before = PortableIssuerState::read();
    assert_eq!(
        reconcile(&domain(), account(), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Unresolved)
    );
    assert_eq!(PortableIssuerState::read(), before);
    assert!(matches!(
        begin(domain(), account(), request(1), policy(), REMOVE_AT),
        Err(E::EpochClosed)
    ));
    let next = start(request_in(1, 2), policy(), REMOVE_AT);
    assert_eq!(challenge(&pending, [8; 32], REMOVE_AT), Err(E::Conflict));
    assert_eq!(finish(&pending, REMOVE_AT), Err(E::Conflict));
    let root = root_at(&next, entry, 2, REMOVE_AT);
    accept_authority(&next, root, REMOVE_AT).unwrap();
    let next_result = finish(&next, REMOVE_AT).unwrap();
    let credential: PortableCredential = next_result.credential.try_into().unwrap();
    let claims = ic_auth::portable::decode_claims(&credential.claims).unwrap();
    assert_eq!(claims.login_started_at_ns, NOW);
    assert_eq!(claims.login_deadline_ns, NOW + policy().login_ttl_ns);
    assert_eq!(claims.issued_at_ns, REMOVE_AT);
}

#[test]
fn automatic_cleanup_allows_more_than_the_retained_capacity_without_reusing_identity() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    enroll();
    let mut now_ns = NOW;
    let mut next_id = 1;
    for _ in 0..4 {
        for _ in 0..MAX_ISSUANCES {
            let epoch = context(&domain()).unwrap().epoch;
            start(request_in(next_id, epoch), policy(), now_ns);
            next_id += 1;
        }
        let journal = load().unwrap();
        assert_eq!(journal.issuances.len(), MAX_ISSUANCES);
        assert_eq!(journal.logins.len(), 1);
        assert_eq!(journal.logins[0].started_at_ns, NOW);
        assert_eq!(journal.logins[0].deadline_ns, NOW + policy().login_ttl_ns);
        now_ns += 70 * SECOND;
    }
    assert!(usize::from(next_id) > MAX_ISSUANCES);
    assert_eq!(context(&domain()).unwrap().epoch, 4);
    assert!(matches!(
        begin(domain(), account(), request(1), policy(), now_ns),
        Err(E::EpochClosed)
    ));
}

#[test]
fn referenced_login_survives_expiry_until_original_credential_retention_is_reclaimed() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let entry = enroll();
    let mut limits = policy();
    limits.login_ttl_ns = 20 * SECOND;
    let pending = start(request(1), limits, NOW);
    let root = root_at(&pending, entry, 1, NOW);
    accept_authority(&pending, root, NOW).unwrap();
    finish(&pending, NOW).unwrap();
    let early = reclaim(&domain(), NOW + 20 * SECOND).unwrap();
    assert_eq!((early.epoch, early.removed_logins), (1, 0));
    assert!(matches!(
        begin(domain(), account(), request(2), limits, NOW + 20 * SECOND),
        Err(E::Expired)
    ));
    let removed = reclaim(&domain(), REMOVE_AT).unwrap();
    assert_eq!(
        (
            removed.epoch,
            removed.removed_issuances,
            removed.removed_logins
        ),
        (2, 1, 1)
    );
    assert!(matches!(
        begin(domain(), account(), request_in(2, 2), limits, REMOVE_AT),
        Err(E::EpochClosed)
    ));
    let mut fresh = request_in(2, 2);
    fresh.login_epoch = 2;
    start(fresh, limits, REMOVE_AT);
    let journal = load().unwrap();
    assert_eq!(journal.logins[0].binding.epoch, 2);
    assert_eq!(journal.logins[0].started_at_ns, REMOVE_AT);
    assert_eq!(journal.logins[0].deadline_ns, REMOVE_AT + 20 * SECOND);
}

#[test]
fn full_live_login_capacity_refuses_new_logins_and_still_allows_existing_login_work() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    enroll();
    let mut limits = policy();
    limits.reconciliation_retention_ns = 1;
    let mut now_ns = NOW;
    for batch in 0..2 {
        let epoch = context(&domain()).unwrap().epoch;
        for offset in 1..=MAX_ISSUANCES {
            let id = u8::try_from(batch * MAX_ISSUANCES + offset).unwrap();
            let mut input = request_in(id, epoch);
            input.login_epoch = epoch;
            input.login_id = [id; 32];
            start(input, limits, now_ns);
        }
        now_ns += 10 * SECOND + 1;
        reclaim(&domain(), now_ns).unwrap();
    }
    assert_eq!(load().unwrap().logins.len(), MAX_LOGINS);
    let epoch = context(&domain()).unwrap().epoch;
    let mut fresh = request_in(99, epoch);
    fresh.login_epoch = epoch;
    fresh.login_id = [99; 32];
    assert!(matches!(
        begin(domain(), account(), fresh, limits, now_ns),
        Err(E::Capacity)
    ));
    let mut renewal = request_in(99, epoch);
    renewal.login_id = [1; 32];
    start(renewal, limits, now_ns);
    let journal = load().unwrap();
    assert_eq!(journal.logins.len(), MAX_LOGINS);
    assert_eq!(journal.issuances[0].login_started_at_ns, NOW);
}

#[test]
fn original_retention_is_immutable_and_clock_rollback_cannot_reclaim_anything() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    enroll();
    pending_at(1, NOW);
    let mut changed = policy();
    changed.reconciliation_retention_ns += SECOND;
    assert!(matches!(
        begin(domain(), account(), request(1), changed, NOW + SECOND),
        Err(E::Conflict)
    ));
    assert_eq!(load().unwrap().issuances[0].remove_at_ns, REMOVE_AT);
    let before = PortableIssuerState::read();
    assert_eq!(reclaim(&domain(), NOW), Err(E::Clock));
    assert_eq!(PortableIssuerState::read(), before);
    let removed = reclaim(&domain(), REMOVE_AT).unwrap();
    assert_eq!(removed.removed_issuances, 1);
    let before = PortableIssuerState::read();
    assert_eq!(reclaim(&domain(), REMOVE_AT - 1), Err(E::Clock));
    assert_eq!(PortableIssuerState::read(), before);
}

#[test]
fn epoch_exhaustion_refuses_before_deleting_retained_history() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    enroll();
    let mut journal = load().unwrap();
    journal.issuance_epoch = u64::MAX;
    commit(&journal).unwrap();
    let mut input = request_in(1, u64::MAX);
    input.login_epoch = u64::MAX;
    start(input, policy(), NOW);
    let journal = load().unwrap();
    assert_eq!(reclaim(&domain(), REMOVE_AT), Err(E::Capacity));
    assert_eq!(load().unwrap(), journal);
}

#[test]
fn already_closed_history_reclaims_without_rotating_or_exhausting_the_current_epoch() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    enroll();
    pending_at(1, NOW);
    let mut journal = load().unwrap();
    journal.issuance_epoch = u64::MAX;
    commit(&journal).unwrap();
    let result = reclaim(&domain(), REMOVE_AT).unwrap();
    assert_eq!((result.epoch, result.removed_issuances), (u64::MAX, 1));
    assert!(matches!(
        begin(domain(), account(), request(1), policy(), REMOVE_AT),
        Err(E::EpochClosed)
    ));
}

#[test]
fn restoration_advances_epoch_and_cannot_reopen_pruned_operation_or_login() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    enroll();
    pending_at(1, NOW);
    reclaim(&domain(), REMOVE_AT).unwrap();
    keys::restore().unwrap();
    assert_eq!(context(&domain()).unwrap().epoch, 3);
    assert!(matches!(
        begin(domain(), account(), request(1), policy(), REMOVE_AT),
        Err(E::EpochClosed)
    ));
    let before = PortableIssuerState::read();
    assert_eq!(
        reconcile(&domain(), account(), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Unresolved)
    );
    assert_eq!(PortableIssuerState::read(), before);
}
