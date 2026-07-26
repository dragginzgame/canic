//! Module: model::env
//!
//! Responsibility: own authoritative initialized environment values.
//! Does not own: environment admission, stable record conversion, or storage access.

use crate::{
    domain::value::Principal,
    ids::{CanisterRole, ComponentSpecId},
};

///
/// ValidatedEnv
///
/// Complete environment state accepted for persistence.
///

#[derive(Clone, Debug)]
pub struct ValidatedEnv {
    pub fleet_root_pid: Principal,
    pub component_spec: Option<ComponentSpecId>,
    pub subnet_pid: Principal,
    pub root_pid: Principal,
    pub canister_role: CanisterRole,
    pub parent_pid: Principal,
}
