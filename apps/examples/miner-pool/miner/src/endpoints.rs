//! The configured pool issues bounded work and may repeat an unacknowledged batch.
use crate::{WORK_CELL, Work};
use canic::{guard::require_parent, topology::Install};

canic::mount_lifecycle! {
    input = Install, max_bytes = 4096, decode = Install::decode,
    install = crate::install, reopen = crate::reopen,
}

canic::mount_readiness_endpoint!(readiness);

fn readiness() -> canic::readiness::State {
    if WORK_CELL.with_borrow(Option::is_some) && canic::topology::parent().is_some() {
        canic::readiness::State::Ready
    } else {
        canic::readiness::State::Unavailable
    }
}

use canic::update;
include!("mine.rs");
