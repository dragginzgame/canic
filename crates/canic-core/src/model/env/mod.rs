//! Module: model::env
//!
//! Responsibility: own authoritative initialized environment values.
//! Does not own: environment admission, stable record conversion, or storage access.

use crate::{
    domain::value::Principal,
    ids::{CanisterRole, SubnetRole},
};

///
/// ValidatedEnv
///
/// Complete environment state accepted for persistence.
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
