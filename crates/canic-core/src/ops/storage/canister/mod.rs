//! Module: ops::storage::canister
//!
//! Responsibility: convert stored canister records into shared read projections.
//! Does not own: stable registry mutation, workflow orchestration, or DTO schemas.
//! Boundary: storage ops conversion shared by registry and child-cache readers.

use crate::{
    cdk::types::Principal, dto::canister::CanisterInfo, storage::canister::CanisterRecord,
};

#[must_use]
pub(super) fn record_to_info(pid: Principal, record: CanisterRecord) -> CanisterInfo {
    CanisterInfo {
        pid,
        role: record.role,
        parent_pid: record.parent_pid,
        module_hash: record.module_hash,
        created_at: record.created_at,
    }
}
