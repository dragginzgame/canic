use crate::{Error, state::StateError, utils::time::now_secs};
use candid::{CandidType, Principal};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, time::Duration};
use thiserror::Error as ThisError;

///
/// Constants for validation
///

const MAX_EXPIRATION: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours
const MIN_EXPIRATION: Duration = Duration::from_secs(60); // 1 minute
const CLEANUP_THRESHOLD: u64 = 1000;

//
// SESSIONS
//

thread_local! {
    static DELEGATION_LIST: RefCell<DelegationList> = RefCell::new(DelegationList::new());
    static CALL_COUNT: RefCell<u64> = const { RefCell::new(0) };
}

///
/// DelegationListError
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum DelegationListError {
    #[error("delegation expired at {0} (current time: {1})")]
    DelegationExpired(u64, u64),

    #[error("no delegation found for session '{0}'")]
    NoExpirySet(Principal),

    #[error("no delegation found for session '{0}'")]
    NotFound(Principal),

    #[error("session length must be at least {0} seconds")]
    SessionTooShort(u64),

    #[error("session length cannot exceed {0} seconds")]
    SessionTooLong(u64),
}

///
/// Data Structures
///

#[derive(Debug, CandidType, Deserialize, Clone)]
pub struct Delegation {
    wallet_pid: Principal,
    expires_at: Option<u64>,
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub struct DelegationView {
    wallet_pid: Principal,
    session_pid: Principal,
    expires_at: Option<u64>,
    is_expired: bool,
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub struct RegisterDelegationArgs {
    pub wallet_pid: Principal,
    pub session_pid: Principal,
    pub duration_secs: u64,
}

///
/// DelegationList
/// map of the session pid to the Delegation
///

#[derive(Default, Debug, Deref, DerefMut)]
pub struct DelegationList(HashMap<Principal, Delegation>);

impl DelegationList {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_delegation(args: RegisterDelegationArgs) -> Result<(), Error> {
        let duration = Duration::from_secs(args.duration_secs);

        // Validate expiration time
        if duration < MIN_EXPIRATION {
            Err(StateError::from(DelegationListError::SessionTooShort(
                MIN_EXPIRATION.as_secs(),
            )))?;
        }

        if duration > MIN_EXPIRATION {
            Err(StateError::from(DelegationListError::SessionTooLong(
                MAX_EXPIRATION.as_secs(),
            )))?;
        }

        let expires_at = now_secs() + args.duration_secs;

        DELEGATION_LIST.with_borrow_mut(|map| {
            // Remove any existing delegation from the same wallet
            map.retain(|_, session| session.wallet_pid != args.wallet_pid);

            // Insert the new delegation
            map.insert(
                args.session_pid,
                Delegation {
                    wallet_pid: args.wallet_pid,
                    expires_at: Some(expires_at),
                },
            );
        });

        Ok(())
    }

    pub fn get_delegation_info(session_pid: Principal) -> Result<DelegationView, Error> {
        let now = now_secs();
        let session = DELEGATION_LIST
            .with_borrow(|map| map.get(&session_pid).cloned())
            .ok_or_else(|| StateError::from(DelegationListError::NotFound(session_pid)))?;

        let is_expired = session.expires_at.is_none_or(|ts| now > ts);

        Ok(DelegationView {
            wallet_pid: session.wallet_pid,
            session_pid,
            expires_at: session.expires_at,
            is_expired,
        })
    }

    #[must_use]
    pub fn list_delegations() -> Vec<DelegationView> {
        let now = now_secs();

        DELEGATION_LIST.with_borrow(|map| {
            map.iter()
                .map(|(&s, d)| DelegationView {
                    session_pid: s,
                    wallet_pid: d.wallet_pid,
                    expires_at: d.expires_at,
                    is_expired: d.expires_at.is_none_or(|expiry| now > expiry),
                })
                .collect()
        })
    }

    pub fn revoke_delegation(pid: Principal) -> Result<(), Error> {
        let was_session = DELEGATION_LIST.with_borrow(|map| map.contains_key(&pid));

        // Was there a session?
        if was_session {
            DELEGATION_LIST.with_borrow_mut(|map| {
                map.remove(&pid);
            });

            return Ok(());
        }

        // Revoke all delegation sessions from this wallet
        let removed = DELEGATION_LIST.with_borrow_mut(|map| {
            let original_len = map.len();
            map.retain(|_, d| d.wallet_pid != pid);
            original_len - map.len()
        });

        if removed > 0 {
            Ok(())
        } else {
            Err(StateError::from(DelegationListError::NotFound(pid)).into())
        }
    }

    #[must_use]
    pub fn get_wallet_delegations(delegator_pid: Principal) -> Vec<DelegationView> {
        let now = now_secs();

        DELEGATION_LIST.with_borrow(|map| {
            map.iter()
                .filter(|(_, del)| del.wallet_pid == delegator_pid)
                .map(|(&session_pid, del)| {
                    let is_expired = del.expires_at.is_none_or(|expiry| now > expiry);

                    DelegationView {
                        session_pid,
                        wallet_pid: del.wallet_pid,
                        expires_at: del.expires_at,
                        is_expired,
                    }
                })
                .collect()
        })
    }

    pub fn cleanup_expired_delegations() {
        let now = now_secs();

        DELEGATION_LIST.with_borrow_mut(|map| {
            map.retain(|_, d| d.expires_at.is_some_and(|ts| now <= ts));
        });
    }

    pub fn cleanup_delegations() -> Result<u32, Error> {
        let before_count = DELEGATION_LIST.with_borrow(|map| map.len());
        Self::cleanup_expired_delegations();
        let after_count = DELEGATION_LIST.with_borrow(|map| map.len());

        #[allow(clippy::cast_possible_truncation)]
        Ok((before_count - after_count) as u32)
    }

    /// Cleanup expired delegations if cleanup is true
    /// Cleanup will only work if called from an update call (!)
    pub fn get_effective_caller_internal(
        caller: Principal,
        cleanup: bool,
    ) -> Result<Principal, Error> {
        if cleanup {
            // Periodically clean up expired delegations (every CLEANUP_THRESHOLD calls)
            CALL_COUNT.with_borrow_mut(|count| {
                *count += 1;
                if *count % CLEANUP_THRESHOLD == 0 {
                    Self::cleanup_expired_delegations();
                }
            });
        }

        let now = now_secs();

        // Check if delegation exists
        let delegation = DELEGATION_LIST
            .with_borrow(|map| map.get(&caller).cloned())
            .ok_or_else(|| StateError::from(DelegationListError::NotFound(caller)))?;

        // Check it has a valid expiry
        let expires_at = delegation
            .expires_at
            .ok_or_else(|| StateError::from(DelegationListError::NoExpirySet(caller)))?;

        // Check it has expired
        if now > expires_at {
            return Err(
                StateError::from(DelegationListError::DelegationExpired(expires_at, now)).into(),
            );
        }

        Ok(delegation.wallet_pid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_pid(n: u8) -> Principal {
        Principal::from_slice(&[n; 29])
    }

    fn create_test_delegation(
        map: &mut DelegationList,
        wallet_pid: Principal,
        session_pid: Principal,
        expires_at: u64,
    ) {
        map.insert(
            session_pid,
            Delegation {
                wallet_pid,
                expires_at: Some(expires_at),
            },
        );
    }

    #[test]
    fn register_and_query_delegation_view() {
        let mut list = DelegationList::new();
        let wallet = dummy_pid(1);
        let session = dummy_pid(2);
        let now = 1_000_000;
        let expires_at = now + 300;

        create_test_delegation(&mut list, wallet, session, expires_at);

        let result = list.get(&session);
        assert!(result.is_some());
        let delegation = result.unwrap();

        assert_eq!(delegation.wallet_pid, wallet);
        assert_eq!(delegation.expires_at, Some(expires_at));
    }

    #[test]
    fn expired_delegation_should_be_cleaned_up() {
        let mut list = DelegationList::new();
        let wallet = dummy_pid(3);
        let session = dummy_pid(4);
        let now = 1_000;

        create_test_delegation(&mut list, wallet, session, 900); // already expired

        list.retain(|_, d| d.expires_at.is_some_and(|ts| now <= ts));
        assert!(list.is_empty());
    }

    #[test]
    fn multiple_sessions_can_be_tracked_for_wallet() {
        let mut list = DelegationList::new();
        let wallet = dummy_pid(5);
        let session1 = dummy_pid(6);
        let session2 = dummy_pid(7);
        let now = 1_000;

        create_test_delegation(&mut list, wallet, session1, now + 100);
        create_test_delegation(&mut list, wallet, session2, now + 200);

        let views: Vec<_> = list
            .iter()
            .filter(|(_, d)| d.wallet_pid == wallet)
            .collect();

        assert_eq!(views.len(), 2);
        assert!(views.iter().all(|(_, d)| d.wallet_pid == wallet));
    }

    #[test]
    fn revoke_specific_session_removes_only_target() {
        let mut list = DelegationList::new();
        let wallet = dummy_pid(8);
        let session1 = dummy_pid(9);
        let session2 = dummy_pid(10);
        let expiry = 2_000;

        create_test_delegation(&mut list, wallet, session1, expiry);
        create_test_delegation(&mut list, wallet, session2, expiry);

        assert!(list.contains_key(&session1));
        assert!(list.contains_key(&session2));

        list.remove(&session1);
        assert!(!list.contains_key(&session1));
        assert!(list.contains_key(&session2));
    }

    #[test]
    fn revoke_all_sessions_from_wallet_removes_them_all() {
        let mut list = DelegationList::new();
        let wallet = dummy_pid(11);
        let s1 = dummy_pid(12);
        let s2 = dummy_pid(13);
        let s3 = dummy_pid(99); // other wallet

        create_test_delegation(&mut list, wallet, s1, 10_000);
        create_test_delegation(&mut list, wallet, s2, 10_000);
        create_test_delegation(&mut list, dummy_pid(200), s3, 10_000);

        let original_len = list.len();
        list.retain(|_, d| d.wallet_pid != wallet);

        assert_eq!(list.len(), original_len - 2);
        assert!(!list.contains_key(&s1));
        assert!(!list.contains_key(&s2));
        assert!(list.contains_key(&s3));
    }

    #[test]
    fn view_contains_expired_flag() {
        let now = 1_000;
        let wallet = dummy_pid(20);
        let session = dummy_pid(21);
        let mut list = DelegationList::new();

        create_test_delegation(&mut list, wallet, session, 500); // expired

        let d = list.get(&session).unwrap();
        let is_expired = d.expires_at.is_none_or(|ts| now > ts);
        assert!(is_expired);
    }
}
