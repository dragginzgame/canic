use super::*;

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn claim_id(id: u64) -> u64 {
    id
}

#[test]
fn claim_pending_returns_bound_when_key_is_already_bound() {
    PlacementBindingRegistryOps::clear_for_test();

    let pid = p(1);
    PlacementBindingRegistryOps::bind("projects", "alpha", pid, 10).expect("initial bind");

    let result =
        PlacementBindingRegistryOps::claim_pending("projects", "alpha", p(9), claim_id(9), 20)
            .expect("claim");

    assert_eq!(
        result,
        PlacementBindingClaimResult::Bound {
            instance_pid: pid,
            bound_at: 10,
        }
    );
}

#[test]
fn bind_promotes_matching_pending_provisional_child() {
    PlacementBindingRegistryOps::clear_for_test();

    let owner_pid = p(1);
    let child_pid = p(2);

    let claim =
        PlacementBindingRegistryOps::claim_pending("projects", "alpha", owner_pid, claim_id(1), 10)
            .expect("initial claim");
    let PlacementBindingClaimResult::Claimed(claim) = claim else {
        panic!("expected new claim");
    };
    PlacementBindingRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        child_pid,
    )
    .expect("attach provisional child");
    PlacementBindingRegistryOps::bind("projects", "alpha", child_pid, 20)
        .expect("bind should promote matching provisional child");

    assert_eq!(
        PlacementBindingRegistryOps::lookup_key("projects", "alpha"),
        Some(child_pid)
    );
}

#[test]
fn lookup_entry_reports_pending_status() {
    PlacementBindingRegistryOps::clear_for_test();

    let owner_pid = p(1);
    PlacementBindingRegistryOps::claim_pending("projects", "alpha", owner_pid, claim_id(1), 10)
        .expect("initial claim");

    assert_eq!(
        PlacementBindingRegistryOps::lookup_entry("projects", "alpha"),
        Some(PlacementBindingStatusResponse::Pending {
            owner_pid,
            created_at: 10,
            provisional_pid: None,
        })
    );
}

#[test]
fn bind_rejects_conflicting_provisional_child() {
    PlacementBindingRegistryOps::clear_for_test();

    let claim =
        PlacementBindingRegistryOps::claim_pending("projects", "alpha", p(1), claim_id(1), 10)
            .expect("initial claim");
    let PlacementBindingClaimResult::Claimed(claim) = claim else {
        panic!("expected new claim");
    };
    PlacementBindingRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        p(2),
    )
    .expect("attach provisional child");

    PlacementBindingRegistryOps::bind("projects", "alpha", p(3), 20)
        .expect_err("conflicting provisional child should fail");
}

#[test]
fn release_stale_pending_removes_stale_entry() {
    PlacementBindingRegistryOps::clear_for_test();

    let owner_pid = p(1);
    let provisional_pid = p(2);
    let claim =
        PlacementBindingRegistryOps::claim_pending("projects", "alpha", owner_pid, claim_id(1), 10)
            .expect("initial claim");
    let PlacementBindingClaimResult::Claimed(claim) = claim else {
        panic!("expected new claim");
    };
    PlacementBindingRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        provisional_pid,
    )
    .expect("attach provisional child");

    let result = PlacementBindingRegistryOps::release_stale_pending_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        10 + PlacementBindingRegistryOps::PENDING_TTL_SECS + 1,
    )
    .expect("release stale pending");

    assert_eq!(
        result,
        PlacementBindingReleaseResult::ReleasedStalePending {
            owner_pid,
            created_at: 10,
            provisional_pid: Some(provisional_pid),
        }
    );
    assert_eq!(
        PlacementBindingRegistryOps::lookup_entry("projects", "alpha"),
        None
    );
}

#[test]
fn release_stale_pending_keeps_fresh_entry_in_place() {
    PlacementBindingRegistryOps::clear_for_test();

    let owner_pid = p(1);
    let claim =
        PlacementBindingRegistryOps::claim_pending("projects", "alpha", owner_pid, claim_id(1), 10)
            .expect("initial claim");

    let PlacementBindingClaimResult::Claimed(claim) = claim else {
        panic!("expected new claim");
    };
    let result = PlacementBindingRegistryOps::release_stale_pending_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        11,
    )
    .expect("fresh pending should not be released");

    assert_eq!(
        result,
        PlacementBindingReleaseResult::PendingRetained {
            owner_pid,
            created_at: 10,
            provisional_pid: None,
        }
    );
    std::assert_matches!(
        PlacementBindingRegistryOps::lookup_entry("projects", "alpha"),
        Some(PlacementBindingStatusResponse::Pending { .. })
    );
}

#[test]
fn release_stale_pending_retains_unknown_child_outcome() {
    PlacementBindingRegistryOps::clear_for_test();

    let owner_pid = p(1);
    let claim =
        PlacementBindingRegistryOps::claim_pending("projects", "alpha", owner_pid, claim_id(1), 10)
            .expect("initial claim");
    let PlacementBindingClaimResult::Claimed(claim) = claim else {
        panic!("expected new claim");
    };

    let result = PlacementBindingRegistryOps::release_stale_pending_if_claim_matches(
        "projects",
        "alpha",
        claim.claim_id,
        10 + PlacementBindingRegistryOps::PENDING_TTL_SECS + 1,
    )
    .expect("unknown child outcome must remain retained");

    assert_eq!(
        result,
        PlacementBindingReleaseResult::PendingRetained {
            owner_pid,
            created_at: 10,
            provisional_pid: None,
        }
    );
    assert!(PlacementBindingRegistryOps::lookup_entry("projects", "alpha").is_some());
}

#[test]
fn pending_claim_cannot_be_stolen_after_ttl_without_explicit_recovery() {
    PlacementBindingRegistryOps::clear_for_test();

    let first =
        PlacementBindingRegistryOps::claim_pending("projects", "alpha", p(1), claim_id(1), 10)
            .expect("initial claim");
    let PlacementBindingClaimResult::Claimed(first_claim) = first else {
        panic!("expected first claim");
    };

    let second = PlacementBindingRegistryOps::claim_pending(
        "projects",
        "alpha",
        p(2),
        claim_id(2),
        10 + PlacementBindingRegistryOps::PENDING_TTL_SECS + 1,
    )
    .expect("existing claim should be returned");
    assert_eq!(
        second,
        PlacementBindingClaimResult::PendingExisting {
            claim_id: first_claim.claim_id,
            owner_pid: first_claim.owner_pid,
            created_at: first_claim.created_at,
            provisional_pid: None,
        }
    );

    let attach_ok = PlacementBindingRegistryOps::set_provisional_pid_if_claim_matches(
        "projects",
        "alpha",
        first_claim.claim_id,
        p(9),
    )
    .expect("original claim owner should retain provisional attach authority");
    assert!(attach_ok);

    let bind_ok = PlacementBindingRegistryOps::bind_if_claim_matches(
        "projects",
        "alpha",
        first_claim.claim_id,
        p(9),
        20,
    )
    .expect("original claim owner should retain bind authority");
    assert!(bind_ok);

    std::assert_matches!(
        PlacementBindingRegistryOps::lookup_state("projects", "alpha"),
        Some(PlacementBindingEntryState::Bound { instance_pid, .. }) if instance_pid == p(9)
    );
}
