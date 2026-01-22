use crate::{
    cdk::types::Principal,
    ids::{BuildNetwork, CanisterRole, SubnetRole},
};
use thiserror::Error as ThisError;

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

///
/// EnvPolicyError
///

#[derive(Debug, ThisError)]
pub enum EnvPolicyError {
    #[error("missing required env fields: {0}")]
    MissingEnvFields(String),
}

fn allow_incomplete_env() -> bool {
    if cfg!(test) {
        return true;
    }

    match option_env!("CANIC_ALLOW_INCOMPLETE_ENV") {
        Some(value) => value == "1",
        None => false,
    }
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

    if missing.is_empty() {
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

        return Ok(ValidatedEnv {
            prime_root_pid,
            subnet_role,
            subnet_pid,
            root_pid,
            canister_role,
            parent_pid,
        });
    }

    if !allow_incomplete_env() {
        return Err(EnvPolicyError::MissingEnvFields(missing.join(", ")));
    }

    let root_pid = raw_env
        .root_pid
        .unwrap_or_else(|| Principal::from_slice(&[0xBB; 29]));
    let subnet_pid = raw_env
        .subnet_pid
        .unwrap_or_else(|| Principal::from_slice(&[0xAA; 29]));
    let canister_role = raw_env
        .canister_role
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("canister_role".to_string()))?;

    Ok(ValidatedEnv {
        prime_root_pid: raw_env.prime_root_pid.unwrap_or(root_pid),
        subnet_role: raw_env.subnet_role.unwrap_or(SubnetRole::PRIME),
        subnet_pid,
        root_pid,
        canister_role,
        parent_pid: raw_env.parent_pid.unwrap_or(root_pid),
    })
}
