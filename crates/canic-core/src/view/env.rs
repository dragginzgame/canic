//! Module: view::env
//!
//! Responsibility: define validated runtime environment projections.
//! Does not own: environment storage, lifecycle restoration, or endpoint DTOs.
//! Boundary: ops and workflow consume this after environment invariants are restored.

use crate::{
    cdk::types::Principal,
    ids::{CanisterRole, SubnetRole},
};

///
/// ValidatedEnv
///
/// Read-only projection of a restored and validated Canic environment.
/// Owned by view and consumed by runtime/env workflows.
///

#[derive(Clone, Debug)]
pub struct ValidatedEnv {
    pub prime_root_pid: Principal,
    pub subnet_role: SubnetRole,
    pub subnet_pid: Principal,
    pub root_pid: Principal,
    pub canister_role: CanisterRole,
    pub parent_pid: Principal,
}
