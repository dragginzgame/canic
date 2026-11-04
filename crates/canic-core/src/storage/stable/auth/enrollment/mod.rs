//! Module: storage::stable::auth::enrollment
//!
//! Responsibility: store bounded Root and receiver enrollment checkpoints.
//! Boundary: ops validates bytes and supplies atomic local generation changes.

use crate::cdk::structures::{DefaultMemoryImpl, StableCell};
use std::cell::RefCell;

pub use crate::storage::stable::auth::{PortableEnrollmentData, PortableEnrollmentRecord};

thread_local! {
    #[cfg(any(test, feature = "auth-portable-issuer"))]
    static ISSUER: RefCell<StableCell<PortableEnrollmentRecord, DefaultMemoryImpl>> = RefCell::new(open_checkpoint(
        crate::memory::keys::AUTH_PORTABLE_ISSUER_V1,
    ));
    #[cfg(any(test, feature = "auth-root-delegation-state"))]
    static ROOT: RefCell<StableCell<PortableEnrollmentRecord, DefaultMemoryImpl>> = RefCell::new(open_checkpoint(
        crate::memory::keys::AUTH_PORTABLE_ROOT_ENROLLMENT_V1,
    ));
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    static RECEIVER: RefCell<StableCell<PortableEnrollmentRecord, DefaultMemoryImpl>> = RefCell::new(open_checkpoint(
        crate::memory::keys::AUTH_PORTABLE_RECEIVER_ENROLLMENT_V1,
    ));
}

/// Public-only issuer journal; the signing key has no stable representation.
#[cfg(any(test, feature = "auth-portable-issuer"))]
pub struct PortableIssuerState;
#[cfg(any(test, feature = "auth-portable-issuer"))]
impl PortableIssuerState {
    pub fn export() -> PortableEnrollmentData {
        PortableEnrollmentData {
            record: ISSUER.with_borrow(|cell| cell.get().clone()),
        }
    }
    pub fn read() -> PortableEnrollmentRecord {
        Self::export().record
    }
    pub fn write(record: PortableEnrollmentRecord) {
        ISSUER.with_borrow_mut(|cell| cell.set(record).expect("commit public issuer journal"));
    }
}

fn open_checkpoint(key: &str) -> StableCell<PortableEnrollmentRecord, DefaultMemoryImpl> {
    let allocation = crate::memory::open(key);
    if ic_storage::Memory::size(&allocation) == 0 {
        StableCell::create(allocation, PortableEnrollmentRecord::default())
            .expect("create admitted enrollment checkpoint")
    } else {
        StableCell::load(allocation, 131_200).expect("load retained enrollment checkpoint")
    }
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
        ROOT.with_borrow_mut(|cell| cell.set(record).expect("commit enrollment checkpoint"));
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
            cell.set(header).expect("commit enrollment generation");
        });
        RECEIVER.with_borrow_mut(|cell| cell.set(record).expect("commit enrollment checkpoint"));
    }
}
