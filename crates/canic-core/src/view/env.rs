use crate::{
    cdk::types::Principal,
    ids::{CanisterRole, SubnetRole},
};

///
/// ValidatedEnv
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
