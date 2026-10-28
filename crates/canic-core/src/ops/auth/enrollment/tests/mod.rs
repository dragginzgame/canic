//! Native producer/installer proofs. IC response authentication remains a runtime boundary.

use super::*;
use crate::{
    ops::storage::auth::application_sessions::{
        ApplicationSessionTestStateGuard, PersistentReceiverPolicy, PersistentReceiverState,
    },
    storage::stable::auth::{
        LocalApplicationAuthorizationState,
        enrollment::{
            PortableEnrollmentRecord, ReceiverPortableEnrollmentState, RootPortableEnrollmentState,
        },
    },
    test::{seams, support::fleet_key},
};

const NOW: u64 = 10_000_000_000;
fn p(id: u8) -> Principal {
    Principal::from_slice(&[id])
}
fn binding() -> EnrollmentBinding {
    EnrollmentBinding {
        authority: p(1),
        receiver: p(2),
        fleet: fleet_key(1),
    }
}
fn domain() -> EnrollmentDomain {
    EnrollmentDomain {
        authority: p(1),
        fleet: fleet_key(1),
    }
}
fn entry(id: u8) -> PortableIssuerEnrollment {
    PortableIssuerEnrollment {
        issuer: p(3),
        key: PortableIssuerKey::Certified {
            certificate_hash: [id; 32],
        },
        not_before_ns: NOW - 1,
        expires_at_ns: NOW + 10_000_000_000,
        grants: vec![PortableEnrollmentGrant {
            role: "app_service".into(),
            scopes: vec!["read".into()],
        }],
        deployment_approval_digest: [7; 32],
        deployment_approval_expires_at_ns: NOW + 20_000_000_000,
    }
}
struct StateGuard(PortableEnrollmentRecord, PortableEnrollmentRecord, u64);
impl StateGuard {
    fn empty() -> Self {
        let guard = Self(
            RootPortableEnrollmentState::read(),
            ReceiverPortableEnrollmentState::read(),
            LocalApplicationAuthorizationState::application_authority_generation(),
        );
        RootPortableEnrollmentState::write(PortableEnrollmentRecord::default());
        ReceiverPortableEnrollmentState::commit(PortableEnrollmentRecord::default(), 0);
        guard
    }
}
impl Drop for StateGuard {
    fn drop(&mut self) {
        RootPortableEnrollmentState::write(self.0.clone());
        ReceiverPortableEnrollmentState::commit(self.1.clone(), self.2);
    }
}
fn approve(entries: Vec<PortableIssuerEnrollment>, retire: Vec<[u8; 32]>) {
    authority::commit_registry(domain(), entries, retire, NOW).unwrap();
}
fn snapshot(challenge: u8) -> PortableEnrollmentSnapshot {
    authority::issue_snapshot(binding(), [challenge; 32], NOW, 1_000_000_000).unwrap()
}

#[test]
fn enrollment_refresh_advances_revision_but_preserves_generation_and_exact_retries() {
    let _lock = seams::lock();
    let _sessions = ApplicationSessionTestStateGuard::empty();
    let _state = StateGuard::empty();
    approve(vec![entry(1)], vec![]);
    let mut receiver = receiver::start_boot(binding(), [9; 32], NOW).unwrap();
    assert_eq!(
        receiver.current_at(NOW),
        Err(PortableEnrollmentError::Unavailable)
    );
    let first = snapshot(9);
    receiver
        .install_authenticated(first.clone(), NOW, NOW + 100)
        .unwrap();
    let generation = LocalApplicationAuthorizationState::application_authority_generation();
    let before = ReceiverPortableEnrollmentState::read();
    let state_before_read = LocalApplicationAuthorizationState::export();
    assert!(receiver.current_at(NOW + 1).is_ok());
    assert_eq!(
        LocalApplicationAuthorizationState::export(),
        state_before_read
    );
    receiver
        .install_authenticated(first.clone(), NOW, NOW + 100)
        .unwrap();
    assert_eq!(ReceiverPortableEnrollmentState::read(), before);
    let refreshed = snapshot(9);
    assert!(refreshed.revision > first.revision);
    assert_eq!(refreshed.generation, first.generation);
    receiver
        .install_authenticated(refreshed.clone(), NOW, NOW + 100)
        .unwrap();
    assert_eq!(
        LocalApplicationAuthorizationState::application_authority_generation(),
        generation
    );
    let before = ReceiverPortableEnrollmentState::read();
    assert_eq!(
        receiver.install_authenticated(first, NOW, NOW + 100),
        Err(PortableEnrollmentError::Rollback)
    );
    let mut equivocation = refreshed;
    equivocation.expires_at_ns += 1;
    assert_eq!(
        receiver.install_authenticated(equivocation, NOW, NOW + 100),
        Err(PortableEnrollmentError::Rollback)
    );
    assert_eq!(ReceiverPortableEnrollmentState::read(), before);
}

#[test]
fn enrollment_boot_and_restoration_require_a_fresh_authenticated_exchange() {
    let _lock = seams::lock();
    let _sessions = ApplicationSessionTestStateGuard::empty();
    let _state = StateGuard::empty();
    approve(vec![entry(1)], vec![]);
    let mut old = receiver::start_boot(binding(), [9; 32], NOW).unwrap();
    let old_reply = snapshot(9);
    old.install_authenticated(old_reply.clone(), NOW, NOW + 100)
        .unwrap();
    let mut restored = receiver::start_boot(binding(), [10; 32], NOW).unwrap();
    assert_eq!(
        old.current_at(NOW),
        Err(PortableEnrollmentError::Unavailable)
    );
    assert_eq!(
        restored.current_at(NOW),
        Err(PortableEnrollmentError::Unavailable)
    );
    assert_eq!(
        restored.install_authenticated(old_reply, NOW, NOW + 100),
        Err(PortableEnrollmentError::Binding)
    );
    restored
        .install_authenticated(snapshot(10), NOW, NOW + 100)
        .unwrap();
    assert!(restored.current_at(NOW).is_ok());
    assert!(matches!(
        receiver::start_boot(binding(), [10; 32], NOW),
        Err(PortableEnrollmentError::Binding)
    ));
    assert_eq!(
        restored.current_at(NOW),
        Err(PortableEnrollmentError::Unavailable)
    );
}

#[test]
fn enrollment_rejects_substitution_late_receipts_expiry_and_backward_clocks() {
    let _lock = seams::lock();
    let _sessions = ApplicationSessionTestStateGuard::empty();
    let _state = StateGuard::empty();
    approve(vec![entry(1)], vec![]);
    let mut receiver = receiver::start_boot(binding(), [9; 32], NOW).unwrap();
    let original = snapshot(9);
    for variant in 0..7 {
        let mut value = original.clone();
        match variant {
            0 => value.authority = p(99),
            1 => value.receiver = p(99),
            2 => value.fleet = fleet_key(2),
            3 => value.boot_challenge = [1; 32],
            4 => value.issued_at_ns = NOW + 1,
            5 => value.expires_at_ns = NOW,
            _ => value.expires_at_ns = NOW + wire::MAX_FRESHNESS_NS + 1,
        }
        assert!(
            receiver
                .install_authenticated(value, NOW, NOW + 100)
                .is_err()
        );
        assert!(ReceiverPortableEnrollmentState::read().bytes.is_empty());
        assert_eq!(
            receiver.current_at(NOW),
            Err(PortableEnrollmentError::Unavailable)
        );
    }
    assert_eq!(
        receiver.install_authenticated(original.clone(), NOW, NOW),
        Err(PortableEnrollmentError::Expired)
    );
    receiver
        .install_authenticated(original.clone(), NOW, NOW + 100)
        .unwrap();
    assert_eq!(
        receiver.current_at(original.expires_at_ns),
        Err(PortableEnrollmentError::Expired)
    );
    assert_eq!(
        receiver.install_authenticated(original.clone(), NOW - 1, NOW + 100),
        Err(PortableEnrollmentError::Clock)
    );
    assert_eq!(
        receiver.current_at(original.expires_at_ns),
        Err(PortableEnrollmentError::Unavailable)
    );
}

#[test]
fn enrollment_changes_advance_local_generation_and_retirement_is_terminal() {
    let _lock = seams::lock();
    let _sessions = ApplicationSessionTestStateGuard::empty();
    let _state = StateGuard::empty();
    approve(vec![entry(1)], vec![]);
    let mut receiver = receiver::start_boot(binding(), [9; 32], NOW).unwrap();
    let first = snapshot(9);
    receiver
        .install_authenticated(first.clone(), NOW, NOW + 100)
        .unwrap();
    let generation = LocalApplicationAuthorizationState::application_authority_generation();
    let mut unversioned = first;
    unversioned.revision += 1;
    unversioned.entries[0].grants[0].scopes.push("write".into());
    assert_eq!(
        receiver.install_authenticated(unversioned, NOW, NOW + 100),
        Err(PortableEnrollmentError::Generation)
    );
    approve(vec![entry(2)], vec![]);
    let retired = snapshot(9);
    assert_eq!(retired.retired_keys, vec![[1; 32]]);
    receiver
        .install_authenticated(retired.clone(), NOW, NOW + 100)
        .unwrap();
    assert_eq!(
        LocalApplicationAuthorizationState::application_authority_generation(),
        generation + 1
    );
    let before = RootPortableEnrollmentState::read();
    assert_eq!(
        authority::commit_registry(domain(), vec![entry(1)], vec![], NOW),
        Err(PortableEnrollmentError::Retired)
    );
    assert_eq!(RootPortableEnrollmentState::read(), before);
    let mut forgotten = retired;
    forgotten.revision += 1;
    forgotten.generation += 1;
    forgotten.retired_keys.clear();
    assert_eq!(
        receiver.install_authenticated(forgotten, NOW, NOW + 100),
        Err(PortableEnrollmentError::Retired)
    );
    let before = ReceiverPortableEnrollmentState::read();
    let mut overflow_header = LocalApplicationAuthorizationState::export();
    overflow_header.authority_generation = u64::MAX;
    LocalApplicationAuthorizationState::import(overflow_header);
    approve(vec![entry(3)], vec![]);
    assert_eq!(
        receiver.install_authenticated(snapshot(9), NOW, NOW + 100),
        Err(PortableEnrollmentError::Capacity)
    );
    assert_eq!(ReceiverPortableEnrollmentState::read(), before);
}

#[test]
fn enrollment_root_rejects_domain_rebinding_counter_overflow_and_bad_grants() {
    let _lock = seams::lock();
    let _sessions = ApplicationSessionTestStateGuard::empty();
    let _state = StateGuard::empty();
    approve(vec![entry(1)], vec![]);
    let before = RootPortableEnrollmentState::read();
    assert_eq!(
        authority::commit_registry(
            EnrollmentDomain {
                authority: p(7),
                fleet: fleet_key(1)
            },
            vec![entry(2)],
            vec![],
            NOW
        ),
        Err(PortableEnrollmentError::Binding)
    );
    let mut invalid = entry(2);
    invalid.grants[0].scopes = vec!["write".into(), "read".into()];
    assert_eq!(
        authority::commit_registry(domain(), vec![invalid], vec![], NOW),
        Err(PortableEnrollmentError::Invalid)
    );
    assert_eq!(RootPortableEnrollmentState::read(), before);
    let mut contents: RegistryContents = decode(&before.bytes).unwrap();
    contents.revision = u64::MAX;
    let exhausted = PortableEnrollmentRecord {
        bytes: encode(&contents).unwrap(),
        observed_at_ns: NOW,
    };
    RootPortableEnrollmentState::write(exhausted.clone());
    assert_eq!(
        authority::issue_snapshot(binding(), [9; 32], NOW, 100),
        Err(PortableEnrollmentError::Capacity)
    );
    assert_eq!(RootPortableEnrollmentState::read(), exhausted);
}

#[test]
fn enrollment_storage_restore_closes_readiness_but_ordinary_cleanup_does_not() {
    let _lock = seams::lock();
    let _sessions = ApplicationSessionTestStateGuard::empty();
    let _state = StateGuard::empty();
    approve(vec![entry(1)], vec![]);
    let mut gate = receiver::start_boot(binding(), [9; 32], NOW).unwrap();
    gate.install_authenticated(snapshot(9), NOW, NOW + 100)
        .unwrap();
    crate::ops::storage::auth::LocalApplicationAuthorizationStateOps::cleanup_application_sessions(
        NOW,
    )
    .unwrap();
    assert!(gate.current_at(NOW).is_ok());
    crate::ops::storage::auth::LocalApplicationAuthorizationStateOps::restore_application_session_state(NOW).unwrap();
    assert_eq!(
        gate.current_at(NOW),
        Err(PortableEnrollmentError::Unavailable)
    );
    let mut restored = receiver::start_boot(binding(), [10; 32], NOW).unwrap();
    assert_eq!(
        restored.current_at(NOW),
        Err(PortableEnrollmentError::Unavailable)
    );
    restored
        .install_authenticated(snapshot(10), NOW, NOW + 100)
        .unwrap();
    assert!(restored.current_at(NOW).is_ok());
    let checkpoint = ReceiverPortableEnrollmentState::read();
    let _owner = PersistentReceiverState::restore(
        PersistentReceiverPolicy {
            fleet: fleet_key(1),
            clock_skew_ns: 1,
            verification_burst: 1,
            verification_refill_interval_ns: 100,
            unknown_logout_burst: 1,
            unknown_logout_refill_interval_ns: 100,
        },
        NOW,
    )
    .unwrap();
    assert_eq!(
        restored.current_at(NOW),
        Err(PortableEnrollmentError::Unavailable)
    );
    assert_eq!(ReceiverPortableEnrollmentState::read(), checkpoint);
}

#[test]
fn enrollment_generation_commit_retains_the_receiver_ledger_and_unknown_logout_budget() {
    let _lock = seams::lock();
    let _sessions = ApplicationSessionTestStateGuard::empty();
    let _state = StateGuard::empty();
    let mut owner = PersistentReceiverState::restore(
        PersistentReceiverPolicy {
            fleet: fleet_key(1),
            clock_skew_ns: 1,
            verification_burst: 1,
            verification_refill_interval_ns: 100,
            unknown_logout_burst: 1,
            unknown_logout_refill_interval_ns: 100,
        },
        NOW - 100,
    )
    .unwrap();
    let logout = owner
        .run(NOW, |store, ledger, _, unknown| {
            Ok(ic_auth::session::logout_session(
                store,
                ledger,
                unknown,
                ic_auth::portable::browser_principal(&[8; 32]),
                NOW,
            ))
        })
        .unwrap();
    assert!(matches!(
        logout,
        ic_auth_protocol_types::LogoutResult::Revoked { .. }
    ));
    let checkpoint = LocalApplicationAuthorizationState::export();
    approve(vec![entry(1)], vec![]);
    let mut gate = receiver::start_boot(binding(), [9; 32], NOW).unwrap();
    gate.install_authenticated(snapshot(9), NOW, NOW + 100)
        .unwrap();
    for changed in [false, true] {
        if changed {
            approve(vec![entry(2)], vec![]);
        }
        gate.install_authenticated(snapshot(9), NOW, NOW + 100)
            .unwrap();
        let expected_generation = if changed { 2 } else { 1 };
        owner
            .run(NOW, |store, ledger, _, unknown| {
                assert_eq!(ledger.occupancy(), (0, 1));
                ic_auth::session::SessionStore::read(store, |reader| {
                    assert_eq!(reader.generation(), expected_generation);
                    Ok(())
                })?;
                assert_eq!(
                    ic_auth::session::logout_session(
                        store,
                        ledger,
                        unknown,
                        ic_auth::portable::browser_principal(&[9; 32]),
                        NOW
                    ),
                    ic_auth_protocol_types::LogoutResult::Unavailable(
                        ic_auth_protocol_types::LogoutUnavailable::Capacity
                    )
                );
                Ok(())
            })
            .unwrap();
        let after = LocalApplicationAuthorizationState::export();
        assert_eq!(after.sessions, checkpoint.sessions);
        assert_eq!(after.replays, checkpoint.replays);
        assert_eq!(after.logins, checkpoint.logins);
    }
}

#[test]
fn enrollment_capacity_and_removal_cannot_erase_terminal_retirements() {
    let _lock = seams::lock();
    let _sessions = ApplicationSessionTestStateGuard::empty();
    let _state = StateGuard::empty();
    approve(vec![entry(1)], vec![]);
    let mut gate = receiver::start_boot(binding(), [9; 32], NOW).unwrap();
    let original = snapshot(9);
    gate.install_authenticated(original.clone(), NOW, NOW + 100)
        .unwrap();
    let mut removal = original;
    removal.entries.clear();
    removal.revision += 1;
    removal.generation += 1;
    assert_eq!(
        gate.install_authenticated(removal, NOW, NOW + 100),
        Err(PortableEnrollmentError::Retired)
    );
    let retired = (0..wire::MAX_RETIRED_KEYS)
        .map(|id| {
            let mut key = [255; 32];
            key[30..].copy_from_slice(&u16::try_from(id).unwrap().to_be_bytes());
            key
        })
        .collect::<Vec<_>>();
    approve(vec![entry(1)], retired);
    let before = RootPortableEnrollmentState::read();
    assert_eq!(
        authority::commit_registry(domain(), vec![], vec![], NOW),
        Err(PortableEnrollmentError::Capacity)
    );
    assert_eq!(RootPortableEnrollmentState::read(), before);
}

#[test]
fn enrollment_out_of_order_or_old_boot_completions_cannot_replace_the_current_view() {
    let _lock = seams::lock();
    let _sessions = ApplicationSessionTestStateGuard::empty();
    let _state = StateGuard::empty();
    approve(vec![entry(1)], vec![]);
    let mut gate = receiver::start_boot(binding(), [9; 32], NOW).unwrap();
    let old_pull = gate.prepare_at(binding(), NOW, NOW + 100).unwrap();
    let old_snapshot = snapshot(9);
    let newer_pull = gate.prepare_at(binding(), NOW, NOW + 100).unwrap();
    let newer_snapshot = snapshot(9);
    gate.finish_at(
        binding(),
        receiver::AuthenticatedPortableEnrollment {
            pull: newer_pull,
            snapshot: newer_snapshot.clone(),
        },
        NOW,
    )
    .unwrap();
    let background = gate.prepare_at(binding(), NOW, NOW + 100).unwrap().fetch();
    assert_eq!(gate.current_at(NOW), Ok(&newer_snapshot));
    drop(background); // No IC call is polled in this native ownership test.
    assert_eq!(
        gate.finish_at(
            binding(),
            receiver::AuthenticatedPortableEnrollment {
                pull: old_pull,
                snapshot: old_snapshot
            },
            NOW
        ),
        Err(PortableEnrollmentError::Rollback)
    );
    assert_eq!(gate.current_at(NOW), Ok(&newer_snapshot));
    let old_boot_pull = gate.prepare_at(binding(), NOW, NOW + 100).unwrap();
    let old_boot_reply = snapshot(9);
    let mut new_boot = receiver::start_boot(binding(), [10; 32], NOW).unwrap();
    assert!(
        new_boot
            .finish_at(
                binding(),
                receiver::AuthenticatedPortableEnrollment {
                    pull: old_boot_pull,
                    snapshot: old_boot_reply
                },
                NOW
            )
            .is_err()
    );
    assert_eq!(
        new_boot.current_at(NOW),
        Err(PortableEnrollmentError::Unavailable)
    );
}
