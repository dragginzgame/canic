mod directory_addressing_seam;
mod pool_selection_seam;
mod registry_policy_seam;
mod retention_seam;
mod topology_invariant_seam;

use crate::cdk::types::Principal;
use std::sync::{Mutex, MutexGuard};

static SEAM_LOCK: Mutex<()> = Mutex::new(());

pub fn lock() -> MutexGuard<'static, ()> {
    SEAM_LOCK.lock().expect("seam tests lock")
}

#[must_use]
pub fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}
