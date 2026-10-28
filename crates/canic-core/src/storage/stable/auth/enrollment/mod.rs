//! Module: storage::stable::auth::enrollment
//!
//! Responsibility: store bounded Root and receiver enrollment checkpoints.
//! Boundary: ops validates bytes and supplies atomic local generation changes.

use crate::cdk::structures::{DefaultMemoryImpl, StableCell};
use std::cell::RefCell;

pub use super::{PortableEnrollmentData, PortableEnrollmentRecord};

thread_local! {
    #[cfg(any(test, feature = "auth-root-delegation-state"))]
    static ROOT: RefCell<StableCell<PortableEnrollmentRecord, DefaultMemoryImpl>> = RefCell::new(StableCell::init(
        crate::memory::open(crate::memory::keys::AUTH_PORTABLE_ROOT_ENROLLMENT_V1), PortableEnrollmentRecord::default()
    ));
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    static RECEIVER: RefCell<StableCell<PortableEnrollmentRecord, DefaultMemoryImpl>> = RefCell::new(StableCell::init(
        crate::memory::open(crate::memory::keys::AUTH_PORTABLE_RECEIVER_ENROLLMENT_V1), PortableEnrollmentRecord::default()
    ));
}

/// Root's retained revision, approved entries and terminal retirements.
#[cfg(any(test, feature = "auth-root-delegation-state"))]
pub struct RootPortableEnrollmentState;
#[cfg(any(test, feature = "auth-root-delegation-state"))]
impl RootPortableEnrollmentState {
    #[must_use]
    pub fn export() -> PortableEnrollmentData {
        PortableEnrollmentData {
            record: ROOT.with_borrow(|cell| cell.get().clone()),
        }
    }
    #[must_use]
    pub fn read() -> PortableEnrollmentRecord {
        Self::export().record
    }
    pub fn write(record: PortableEnrollmentRecord) {
        ROOT.with_borrow_mut(|cell| cell.set(record));
    }
}

/// Receiver checkpoint, committed in the same message as session invalidation.
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub struct ReceiverPortableEnrollmentState;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
impl ReceiverPortableEnrollmentState {
    pub fn export() -> PortableEnrollmentData {
        PortableEnrollmentData {
            record: RECEIVER.with_borrow(|cell| cell.get().clone()),
        }
    }
    pub fn read() -> PortableEnrollmentRecord {
        Self::export().record
    }
    pub fn commit(record: PortableEnrollmentRecord, generation: u64) {
        super::LOCAL_APPLICATION_AUTHORIZATION_STATE.with_borrow_mut(|cell| {
            let mut header = cell.get().clone();
            header.application_authority_generation = generation;
            cell.set(header);
        });
        RECEIVER.with_borrow_mut(|cell| cell.set(record));
    }
}
