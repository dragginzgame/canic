use crate::{
    cdk::types::Principal,
    ids::{BuildNetwork, CanisterRole, SubnetRole},
};
use thiserror::Error as ThisError;

pub use crate::view::env::ValidatedEnv;

///
/// EnvInput
///

#[derive(Clone, Debug)]
pub struct EnvInput {
    pub prime_root_pid: Option<Principal>,
    pub subnet_role: Option<SubnetRole>,
    pub subnet_pid: Option<Principal>,
    pub root_pid: Option<Principal>,
    pub canister_role: Option<CanisterRole>,
    pub parent_pid: Option<Principal>,
}

///
/// EnvPolicyError
///

#[derive(Debug, ThisError)]
pub enum EnvPolicyError {
    #[error("missing required env fields: {0}")]
    MissingEnvFields(String),
}

pub fn validate_or_default(
    _network: BuildNetwork,
    raw_env: EnvInput,
) -> Result<ValidatedEnv, EnvPolicyError> {
    let mut missing = Vec::new();
    if raw_env.prime_root_pid.is_none() {
        missing.push("prime_root_pid");
    }
    if raw_env.subnet_role.is_none() {
        missing.push("subnet_role");
    }
    if raw_env.subnet_pid.is_none() {
        missing.push("subnet_pid");
    }
    if raw_env.root_pid.is_none() {
        missing.push("root_pid");
    }
    if raw_env.canister_role.is_none() {
        missing.push("canister_role");
    }
    if raw_env.parent_pid.is_none() {
        missing.push("parent_pid");
    }

    if !missing.is_empty() {
        return Err(EnvPolicyError::MissingEnvFields(missing.join(", ")));
    }

    let prime_root_pid = raw_env
        .prime_root_pid
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("prime_root_pid".to_string()))?;
    let subnet_role = raw_env
        .subnet_role
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("subnet_role".to_string()))?;
    let subnet_pid = raw_env
        .subnet_pid
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("subnet_pid".to_string()))?;
    let root_pid = raw_env
        .root_pid
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("root_pid".to_string()))?;
    let canister_role = raw_env
        .canister_role
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("canister_role".to_string()))?;
    let parent_pid = raw_env
        .parent_pid
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("parent_pid".to_string()))?;

    Ok(ValidatedEnv {
        prime_root_pid,
        subnet_role,
        subnet_pid,
        root_pid,
        canister_role,
        parent_pid,
    })
}
